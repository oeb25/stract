// Cuely is an open source web search engine.
// Copyright (C) 2022 Cuely ApS
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::collections::BTreeMap;

use chrono::{NaiveDate, NaiveDateTime, Utc};
use itertools::{intersperse, Itertools};
use serde::{Deserialize, Serialize};

use crate::{
    entity_index::{entity::Span, StoredEntity},
    inverted_index::{self, RetrievedWebpage},
    searcher::{self, LocalSearcher},
    webpage::Url,
};

pub fn initial(
    result: searcher::local::InitialWebsiteResult,
    local_searcher: &LocalSearcher,
) -> InitialWebsiteResult {
    let entity = result
        .entity
        .map(|entity| DisplayedEntity::from(entity, local_searcher));

    InitialWebsiteResult {
        spell_corrected_query: result.spell_corrected_query,
        websites: result.websites,
        entity,
    }
}

pub fn retrieve(result: Vec<RetrievedWebpage>, searcher: &LocalSearcher) -> Vec<DisplayedWebpage> {
    result
        .into_iter()
        .map(|mut webpage| {
            webpage.primary_image = webpage.primary_image.and_then(|image| {
                if searcher
                    .primary_image(image.uuid.clone().to_string())
                    .is_some()
                {
                    Some(image)
                } else {
                    None
                }
            });

            let url: Url = webpage.url.clone().into();
            webpage.favicon = searcher.favicon(&url.site().to_string().into());
            webpage
        })
        .map(DisplayedWebpage::from)
        .collect()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitialWebsiteResult {
    pub spell_corrected_query: Option<String>,
    pub websites: inverted_index::InitialSearchResult,
    pub entity: Option<DisplayedEntity>,
}

pub fn html_escape(s: &str) -> String {
    html_escape::decode_html_entities(s)
        .chars()
        .filter(|c| !matches!(c, '<' | '>' | '&'))
        .collect()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DisplayedWebpage {
    pub title: String,
    pub url: String,
    pub site: String,
    pub favicon_base64: String,
    pub domain: String,
    pub pretty_url: String,
    pub snippet: String,
    pub body: String,
    pub primary_image_uuid: Option<String>,
    pub last_updated: Option<String>,
}

fn prettify_url(url: &Url) -> String {
    let mut pretty_url = url.strip_query().to_string();

    if pretty_url.ends_with('/') {
        pretty_url = pretty_url.chars().take(pretty_url.len() - 1).collect();
    }

    let protocol = Url::from(pretty_url.clone()).protocol().to_string() + "://";
    pretty_url = Url::from(pretty_url.clone())
        .strip_protocol()
        .replace('/', " › ");
    pretty_url = protocol + &pretty_url;

    pretty_url
}

fn prettify_date(date: NaiveDateTime) -> String {
    let current_time = Utc::now().naive_utc();
    let diff = current_time.signed_duration_since(date);

    let num_hours = diff.num_hours() + 1;
    if num_hours < 24 {
        if num_hours <= 1 {
            return "1 hour ago".to_string();
        }

        return format!("{num_hours} hours ago");
    }

    let num_days = diff.num_days();
    if num_days < 30 {
        if num_days <= 1 {
            return "1 day ago".to_string();
        }

        return format!("{num_days} days ago");
    }

    format!("{}", date.format("%d. %b. %Y"))
}

impl From<RetrievedWebpage> for DisplayedWebpage {
    fn from(webpage: RetrievedWebpage) -> Self {
        let last_updated = webpage.updated_time.map(prettify_date);

        let url: Url = webpage.url.clone().into();
        let domain = url.domain().to_string();
        let pretty_url = prettify_url(&url);

        let title = html_escape(&webpage.title);

        let favicon_bytes = webpage
            .favicon
            .map(|favicon| favicon.as_raw_bytes())
            .unwrap_or_else(|| include_bytes!("../frontend/dist/images/globe.png").to_vec());

        Self {
            title,
            site: url.site().to_string(),
            url: webpage.url,
            pretty_url,
            domain,
            favicon_base64: base64::encode(favicon_bytes),
            snippet: webpage.snippet, // snippet has already been html-escaped.
            body: webpage.body,
            primary_image_uuid: webpage.primary_image.map(|image| image.uuid.to_string()),
            last_updated,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DisplayedEntity {
    pub title: String,
    pub small_abstract: String,
    pub image_base64: Option<String>,
    pub related_entities: Vec<DisplayedEntity>,
    pub info: Vec<(String, String)>,
}

fn prepare_info(info: BTreeMap<String, Span>, searcher: &LocalSearcher) -> Vec<(String, String)> {
    let mut info: Vec<_> = info.into_iter().collect();

    info.sort_by(|(a, _), (b, _)| {
        searcher
            .attribute_occurrence(b)
            .unwrap_or(0)
            .cmp(&searcher.attribute_occurrence(a).unwrap_or(0))
    });

    info.into_iter()
        .map(|(key, value)| {
            let mut value = entity_link_to_html(value, 150).replace('*', "•");

            if value.starts_with('•') || value.starts_with("\n•") {
                if let Some(first_bullet) = value.find('•') {
                    value = value.chars().skip(first_bullet + 1).collect();
                }
            }

            let value = maybe_prettify_entity_date(value);

            (key.replace('_', " "), value)
        })
        .filter(|(key, _)| {
            !matches!(
                key.as_str(),
                "caption"
                    | "image size"
                    | "label"
                    | "landscape"
                    | "signature"
                    | "name"
                    | "website"
                    | "logo"
                    | "image caption"
            )
        })
        .take(5)
        .collect()
}

impl DisplayedEntity {
    pub fn from(entity: StoredEntity, searcher: &LocalSearcher) -> Self {
        let entity_abstract = Span {
            text: entity.entity_abstract,
            links: entity.links,
        };

        let small_abstract = entity_link_to_html(entity_abstract, 300);

        let image_base64 = entity
            .image
            .and_then(|image| searcher.entity_image(image))
            .map(|image| base64::encode(image.as_raw_bytes()));

        Self {
            title: entity.title,
            small_abstract,
            image_base64,
            related_entities: entity
                .related_entities
                .into_iter()
                .map(|entity| DisplayedEntity::from(entity, searcher))
                .collect(),
            info: prepare_info(entity.info, searcher),
        }
    }
}

fn maybe_prettify_entity_date(value: String) -> String {
    if let Ok(date) = NaiveDate::parse_from_str(value.trim(), "%Y %-m %-d") {
        return format!("{}", date.format("%d/%m/%Y"));
    }

    if value.split_whitespace().count() == 6 {
        let first_date = NaiveDate::parse_from_str(
            itertools::intersperse(value.split_whitespace().take(3), " ")
                .collect::<String>()
                .as_str(),
            "%Y %-m %-d",
        );
        let second_date = NaiveDate::parse_from_str(
            itertools::intersperse(value.split_whitespace().skip(3), " ")
                .collect::<String>()
                .as_str(),
            "%Y %-m %-d",
        );

        if let (Ok(first_date), Ok(second_date)) = (first_date, second_date) {
            // the dates are reversed from the parser, so we need to return second_date before first_date
            return format!("{}", second_date.format("%d/%m/%Y"))
                + " - "
                + format!("{}", first_date.format("%d/%m/%Y")).as_str();
        }
    }

    value
}

pub fn thousand_sep_number(num: usize) -> String {
    let s = num.to_string();
    let c = s.chars().rev().chunks(3);
    let chunks = c.into_iter().map(|chunk| {
        chunk
            .into_iter()
            .collect::<Vec<char>>()
            .into_iter()
            .rev()
            .collect::<String>()
    });

    intersperse(
        chunks.collect::<Vec<_>>().into_iter().rev(),
        ".".to_string(),
    )
    .collect()
}

fn entity_link_to_html(span: Span, trunace_to: usize) -> String {
    let mut s = span.text;

    let truncated = s.len() > trunace_to;
    if truncated {
        s = s.chars().take(trunace_to).collect();
    }

    let chars = s.chars();
    let num_chars = chars.clone().count();

    let mut res = String::new();

    let mut last_link_end = 0;
    for link in span.links {
        if link.start > num_chars {
            break;
        }

        res += chars
            .clone()
            .skip(last_link_end)
            .take(link.start - last_link_end)
            .collect::<String>()
            .as_str();

        let link_text: String = chars
            .clone()
            .skip(link.start)
            .take(link.end - link.start)
            .collect();

        res += format!(
            "<a href=\"https://en.wikipedia.org/wiki/{}\">",
            link.target.replace(' ', "_")
        )
        .as_str();

        res += link_text.as_str();

        res += "</a>";

        last_link_end = link.end;
    }

    res += chars
        .clone()
        .skip(last_link_end)
        .collect::<String>()
        .as_str();

    if truncated {
        res += "...";
    }

    res
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use crate::entity_index::entity::Link;

    use super::*;

    #[test]
    fn prettify_date_in_hours() {
        let date = Utc::now().naive_utc();
        assert_eq!(prettify_date(date), "1 hour ago".to_string());

        let date = (Utc::now() - chrono::Duration::seconds(4000)).naive_utc();
        assert_eq!(prettify_date(date), "2 hours ago".to_string());
    }

    #[test]
    fn prettify_date_days() {
        let date = (Utc::now() - chrono::Duration::days(1)).naive_utc();
        assert_eq!(prettify_date(date), "1 day ago".to_string());

        let date = (Utc::now() - chrono::Duration::days(2)).naive_utc();
        assert_eq!(prettify_date(date), "2 days ago".to_string());
    }

    #[test]
    fn prettify_date_rest() {
        let date = DateTime::parse_from_rfc3339("1996-12-19T16:39:57-08:00")
            .unwrap()
            .naive_local();
        assert_eq!(prettify_date(date), "19. Dec. 1996".to_string());
    }

    #[test]
    fn simple_link_to_html() {
        assert_eq!(
            entity_link_to_html(
                Span {
                    text: "some text with a link".to_string(),
                    links: vec![Link {
                        start: 5,
                        end: 9,
                        target: "text article".to_string()
                    }]
                },
                10000
            ),
            "some <a href=\"https://en.wikipedia.org/wiki/text_article\">text</a> with a link"
                .to_string()
        );
    }

    #[test]
    fn truncated_link_to_html() {
        assert_eq!(
            entity_link_to_html(
                Span {
                    text: "some text".to_string(),
                    links: vec![Link {
                        start: 5,
                        end: 9,
                        target: "text article".to_string()
                    }]
                },
                7
            ),
            "some <a href=\"https://en.wikipedia.org/wiki/text_article\">te</a>...".to_string()
        );
    }

    #[test]
    fn einstein_date() {
        assert_eq!(
            maybe_prettify_entity_date("1879 3 14 ".to_string()),
            "14/03/1879".to_string()
        );
    }

    #[test]
    fn entity_date_span_prettify() {
        assert_eq!(
            maybe_prettify_entity_date(" 1999 5 27 1879 3 14  ".to_string()),
            "14/03/1879 - 27/05/1999".to_string()
        );
    }

    #[test]
    fn sep_number() {
        assert_eq!(thousand_sep_number(0), "0".to_string());
        assert_eq!(thousand_sep_number(10), "10".to_string());
        assert_eq!(thousand_sep_number(100), "100".to_string());
        assert_eq!(thousand_sep_number(1000), "1.000".to_string());
        assert_eq!(thousand_sep_number(10_000), "10.000".to_string());
        assert_eq!(thousand_sep_number(100_000), "100.000".to_string());
        assert_eq!(thousand_sep_number(512_854), "512.854".to_string());
        assert_eq!(thousand_sep_number(9_512_854), "9.512.854".to_string());
    }
}
