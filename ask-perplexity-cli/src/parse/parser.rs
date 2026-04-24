use crate::config::{
    phase_by_icon, ANSWER_CONTAINER, SOURCES_CONTAINER, SOURCE_ITEM, SOURCE_SNIPPET,
    SOURCE_TITLE, STEP_ITEM, STEP_PHASE_ICON, STEP_QUERY,
};
use crate::types::{PerplexityResult, Phase, RawHtml, Source, Step};
use scraper::{ElementRef, Html, Selector};
use url::Url;

#[derive(Debug)]
pub struct ParseOptions<'a> {
    pub url: Option<&'a str>,
    pub deep: bool,
    pub raw: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("answer container not found")]
    AnswerContainerMissing,
    #[error("answer container empty")]
    AnswerEmpty,
    #[error("internal selector error: {0}")]
    Selector(String),
}

fn compile(sel: &str) -> Result<Selector, ParseError> {
    Selector::parse(sel).map_err(|e| ParseError::Selector(format!("{:?}", e)))
}

/// Parse Perplexity's rendered HTML into a structured result. Mirrors
/// `providers/perplexity/parse.js` from Plan 1 so the same fixtures can
/// verify both implementations.
pub fn parse(html: &str, opts: ParseOptions<'_>) -> Result<PerplexityResult, ParseError> {
    let document = Html::parse_document(html);

    let answer_sel = compile(ANSWER_CONTAINER)?;
    let answer_node = document
        .select(&answer_sel)
        .last()
        .ok_or(ParseError::AnswerContainerMissing)?;
    let answer = answer_node.text().collect::<String>().trim().to_string();
    if answer.is_empty() {
        return Err(ParseError::AnswerEmpty);
    }

    let sources = extract_sources(&document)?;
    let steps = if opts.deep {
        Some(extract_steps(&document)?)
    } else {
        None
    };
    let raw = if opts.raw {
        let sources_sel = compile(SOURCES_CONTAINER)?;
        let sources_html = document
            .select(&sources_sel)
            .next()
            .map(|el| el.html())
            .unwrap_or_default();
        Some(RawHtml {
            answer_html: answer_node.html(),
            sources_html,
        })
    } else {
        None
    };
    let thread_id = opts.url.and_then(extract_thread_id);

    Ok(PerplexityResult {
        answer,
        sources,
        thread_id,
        steps,
        raw,
        job_id: None,
    })
}

fn extract_sources(document: &Html) -> Result<Vec<Source>, ParseError> {
    let item_sel = compile(SOURCE_ITEM)?;
    let title_sel = compile(SOURCE_TITLE)?;
    let snippet_sel = compile(SOURCE_SNIPPET)?;

    let mut out = Vec::new();
    for (i, card) in document.select(&item_sel).enumerate() {
        let Some(href) = card.value().attr("href") else {
            continue;
        };
        let Ok(parsed_url) = Url::parse(href) else {
            continue;
        };
        let domain = parsed_url.host_str().unwrap_or("").to_string();
        let title = card
            .select(&title_sel)
            .next()
            .map(|n| n.text().collect::<String>().trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| domain.clone());
        let snippet = card
            .select(&snippet_sel)
            .next()
            .map(|n| n.text().collect::<String>().trim().to_string())
            .filter(|s| !s.is_empty());
        out.push(Source {
            index: i + 1,
            title,
            url: href.to_string(),
            domain,
            snippet,
        });
    }
    Ok(out)
}

fn extract_steps(document: &Html) -> Result<Vec<Step>, ParseError> {
    let button_sel = compile(STEP_ITEM)?;
    let query_sel = compile(STEP_QUERY)?;
    let icon_sel = compile(STEP_PHASE_ICON)?;

    let mut out = Vec::new();
    for button in document.select(&button_sel) {
        // Heuristic matches parse.js: step buttons contain a div with the
        // distinctive text class cascade. Other buttons won't have this.
        let Some(query_node) = button.select(&query_sel).next() else {
            continue;
        };
        let query = query_node.text().collect::<String>().trim().to_string();
        if query.is_empty() {
            continue;
        }
        let icon_ref = button
            .select(&icon_sel)
            .next()
            .and_then(|u| {
                u.value()
                    .attr("xlink:href")
                    .or_else(|| u.value().attr("href"))
            })
            .unwrap_or("");
        out.push(Step {
            query,
            phase: phase_by_icon(icon_ref),
        });
    }
    Ok(out)
}

fn extract_thread_id(url: &str) -> Option<String> {
    let u = Url::parse(url).ok()?;
    let mut segments = u.path_segments()?;
    if segments.next()? != "search" {
        return None;
    }
    let slug = segments.next()?;
    if slug.is_empty() {
        None
    } else {
        Some(slug.to_string())
    }
}
