// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! Result formatting and HTML stripping.
//!
//! Formats raw search results, definitions, and other domain models
//! into human-readable text suitable for LLM consumption. Strips
//! OpenGrok's `<b>` markup when requested.

use crate::domain::*;

// ---------------------------------------------------------------------------
// HTML stripping
// ---------------------------------------------------------------------------

/// Removes `<b>` and `</b>` tags from a string.
///
/// OpenGrok wraps search matches in `<b>Match</b>`. This function
/// strips those tags for cleaner LLM input.
#[must_use]
pub fn strip_html_tags(text: &str) -> String {
    text.replace("<b>", "").replace("</b>", "")
}

// ---------------------------------------------------------------------------
// Result formatter
// ---------------------------------------------------------------------------

/// Configuration for result formatting.
#[derive(Debug, Clone)]
pub struct FormatterConfig {
    /// Whether to strip `<b>` tags from search result lines.
    pub strip_html: bool,
    /// Prefix for each result line.
    pub line_prefix: String,
}

impl Default for FormatterConfig {
    fn default() -> Self {
        Self {
            strip_html: true,
            line_prefix: "  ".into(),
        }
    }
}

/// Formats search results and other domain objects into
/// human-readable text.
#[derive(Debug, Clone, Default)]
pub struct ResultFormatter {
    config: FormatterConfig,
}

impl ResultFormatter {
    /// Creates a new formatter with the given configuration.
    #[must_use]
    pub fn new(config: FormatterConfig) -> Self {
        Self { config }
    }

    /// Formats search results.
    ///
    /// Output format:
    /// ```text
    /// Found N result(s) across M file(s):
    /// ### /path/to/file
    ///   Line 42: matched line text (function)
    ///   Line 99: another match
    ///
    /// [has more results — use start=N for next page]
    /// ```
    #[must_use]
    pub fn format_search(&self, results: &SearchResults) -> String {
        let file_count = results.hits_by_file.len();
        let hit_count: usize = results.hits_by_file.iter().map(|f| f.hits.len()).sum();

        let mut output = String::new();

        output.push_str(&format!(
            "Found {hit_count} result(s) across {file_count} file(s):\n"
        ));

        for file_hits in &results.hits_by_file {
            output.push_str(&format!("\n### {}\n", file_hits.path));

            for hit in &file_hits.hits {
                let line_text = if self.config.strip_html {
                    strip_html_tags(&hit.line)
                } else {
                    hit.line.clone()
                };

                let tag_suffix = if hit.tag.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", hit.tag)
                };

                output.push_str(&format!(
                    "{}Line {}: {}{}\n",
                    self.config.line_prefix, hit.line_number, line_text, tag_suffix
                ));
            }
        }

        if results.has_more() {
            let next_start = results.end_document + 1;
            output.push_str(&format!(
                "\n[has more results — use start={next_start} for next page]\n"
            ));
        }

        output
    }

    /// Formats a list of file definitions.
    #[must_use]
    pub fn format_definitions(&self, defs: &[FileDefinition]) -> String {
        if defs.is_empty() {
            return "No definitions found.\n".into();
        }

        let mut output = format!("Found {} definition(s):\n", defs.len());

        for def in defs {
            output.push_str(&format!(
                "{}Line {}: {} — `{}`",
                self.config.line_prefix, def.line, def.symbol, def.signature
            ));
            if let Some(ref ns) = def.namespace {
                output.push_str(&format!(" [namespace: {ns}]"));
            }
            output.push('\n');
        }

        output
    }

    /// Formats directory listing.
    #[must_use]
    pub fn format_directory(&self, entries: &[DirectoryEntry]) -> String {
        if entries.is_empty() {
            return "Directory is empty.\n".into();
        }

        let mut output = format!("Found {} entries:\n", entries.len());
        for entry in entries {
            let kind = if entry.is_directory { "📁" } else { "📄" };
            output.push_str(&format!(
                "{}{}  {}\n",
                self.config.line_prefix, kind, entry.path
            ));
        }
        output
    }

    /// Formats a history response.
    #[must_use]
    pub fn format_history(&self, history: &HistoryResponse) -> String {
        if history.entries.is_empty() {
            return format!("No history found (total: {} revisions).\n", history.total);
        }

        let mut output = format!(
            "Showing {} of {} revisions:\n",
            history.count, history.total
        );

        for entry in &history.entries {
            output.push_str(&format!(
                "{}`{}` by {} — {}\n",
                self.config.line_prefix, entry.revision, entry.author, entry.message
            ));
        }

        output
    }

    /// Formats annotation (blame) entries.
    #[must_use]
    pub fn format_annotation(&self, entries: &[AnnotationEntry]) -> String {
        if entries.is_empty() {
            return "No annotation found.\n".into();
        }

        let mut output = format!("Found {} annotated lines:\n", entries.len());
        for (i, entry) in entries.iter().enumerate() {
            output.push_str(&format!(
                "{}Line {}: {} — {} ({})\n",
                self.config.line_prefix,
                i + 1,
                entry.revision,
                entry.author,
                entry.description,
            ));
        }
        output
    }

    /// Formats a list of project names.
    #[must_use]
    pub fn format_projects(&self, projects: &[String]) -> String {
        if projects.is_empty() {
            return "No projects found.\n".into();
        }

        let mut output = format!("Found {} project(s):\n", projects.len());
        for p in projects {
            output.push_str(&format!("{}- {}\n", self.config.line_prefix, p));
        }
        output
    }

    /// Formats suggestions.
    #[must_use]
    pub fn format_suggestions(&self, suggestions: &[Suggestion]) -> String {
        if suggestions.is_empty() {
            return "No suggestions found.\n".into();
        }

        let mut output = format!("Found {} suggestion(s):\n", suggestions.len());
        for s in suggestions {
            let score = s
                .score
                .map_or_else(String::new, |sc| format!(" (score: {sc})"));
            output.push_str(&format!(
                "{}- {}{}\n",
                self.config.line_prefix, s.phrase, score
            ));
        }
        output
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- strip_html_tags ----------------------------------------------------

    #[test]
    fn strip_b_tags() {
        assert_eq!(strip_html_tags("fn <b>main</b>()"), "fn main()");
    }

    #[test]
    fn strip_multiple_tags() {
        assert_eq!(strip_html_tags("<b>hello</b> <b>world</b>"), "hello world");
    }

    #[test]
    fn strip_no_tags_noop() {
        assert_eq!(strip_html_tags("plain text"), "plain text");
    }

    // -- format_search ------------------------------------------------------

    fn make_hit(line_number: u32, line: &str, tag: &str) -> SearchHit {
        SearchHit {
            line_number: LineNumber(line_number),
            line: line.into(),
            tag: tag.into(),
        }
    }

    fn make_results(hits: Vec<FileHits>, result_count: u32, end_document: u32) -> SearchResults {
        SearchResults {
            result_count,
            start_document: 0,
            end_document,
            duration_ms: 10,
            hits_by_file: hits,
        }
    }

    #[test]
    fn format_search_single_hit() {
        let formatter = ResultFormatter::default();
        let results = make_results(
            vec![FileHits {
                path: "/src/main.rs".into(),
                hits: vec![make_hit(42, "fn <b>main</b>() {}", "function")],
            }],
            1,
            0,
        );

        let output = formatter.format_search(&results);
        assert!(output.contains("Found 1 result(s) across 1 file(s)"));
        assert!(output.contains("### /src/main.rs"));
        assert!(output.contains("Line 42: fn main() {} (function)"));
    }

    #[test]
    fn format_search_strips_html() {
        let formatter = ResultFormatter::new(FormatterConfig {
            strip_html: true,
            ..Default::default()
        });
        let results = make_results(
            vec![FileHits {
                path: "/a.rs".into(),
                hits: vec![make_hit(1, "<b>hello</b>", "")],
            }],
            1,
            0,
        );
        let output = formatter.format_search(&results);
        assert!(!output.contains("<b>"));
        assert!(output.contains("hello"));
    }

    #[test]
    fn format_search_keeps_html_when_disabled() {
        let formatter = ResultFormatter::new(FormatterConfig {
            strip_html: false,
            ..Default::default()
        });
        let results = make_results(
            vec![FileHits {
                path: "/a.rs".into(),
                hits: vec![make_hit(1, "<b>hello</b>", "")],
            }],
            1,
            0,
        );
        let output = formatter.format_search(&results);
        assert!(output.contains("<b>hello</b>"));
    }

    #[test]
    fn format_search_shows_has_more() {
        let formatter = ResultFormatter::default();
        // 50 results, only returned first page (0..9 of 50)
        let results = SearchResults {
            result_count: 50,
            start_document: 0,
            end_document: 9,
            duration_ms: 5,
            hits_by_file: vec![],
        };
        let output = formatter.format_search(&results);
        assert!(output.contains("has more results"));
        assert!(output.contains("use start=10"));
    }

    #[test]
    fn format_search_no_more_when_complete() {
        let formatter = ResultFormatter::default();
        let results = SearchResults {
            result_count: 5,
            start_document: 0,
            end_document: 4,
            duration_ms: 5,
            hits_by_file: vec![],
        };
        let output = formatter.format_search(&results);
        assert!(!output.contains("has more results"));
    }

    #[test]
    fn format_definitions_output() {
        let formatter = ResultFormatter::default();
        let defs = vec![FileDefinition {
            def_type: "function".into(),
            signature: "fn foo()".into(),
            text: "pub fn foo()".into(),
            symbol: "foo".into(),
            line_start: 10,
            line_end: 15,
            line: 10,
            namespace: None,
        }];
        let output = formatter.format_definitions(&defs);
        assert!(output.contains("Found 1 definition(s)"));
        assert!(output.contains("foo"));
        assert!(output.contains("fn foo()"));
    }

    #[test]
    fn format_definitions_empty() {
        let formatter = ResultFormatter::default();
        let output = formatter.format_definitions(&[]);
        assert_eq!(output, "No definitions found.\n");
    }

    #[test]
    fn format_directory_output() {
        let formatter = ResultFormatter::default();
        let entries = vec![
            DirectoryEntry {
                path: "/src".into(),
                is_directory: true,
                num_lines: 0,
                loc: 0,
                date: None,
                description: None,
                size: None,
            },
            DirectoryEntry {
                path: "/src/main.rs".into(),
                is_directory: false,
                num_lines: 100,
                loc: 80,
                date: Some(1712345678000),
                description: None,
                size: Some(2048),
            },
        ];
        let output = formatter.format_directory(&entries);
        assert!(output.contains("📁"));
        assert!(output.contains("📄"));
        assert!(output.contains("/src"));
        assert!(output.contains("/src/main.rs"));
    }

    #[test]
    fn format_history_output() {
        let formatter = ResultFormatter::default();
        let history = HistoryResponse {
            entries: vec![HistoryEntry {
                revision: "abc123".into(),
                author: "dev".into(),
                date: 1700000000000,
                message: "fix bug".into(),
                tags: vec!["v1.0".into()],
                files: vec!["src/main.rs".into()],
            }],
            start: 0,
            count: 1,
            total: 100,
        };
        let output = formatter.format_history(&history);
        assert!(output.contains("Showing 1 of 100 revisions"));
        assert!(output.contains("abc123"));
        assert!(output.contains("dev"));
        assert!(output.contains("fix bug"));
    }

    #[test]
    fn format_annotation_output() {
        let formatter = ResultFormatter::default();
        let entries = vec![AnnotationEntry {
            revision: "def456".into(),
            author: "dev2".into(),
            description: "add feature".into(),
            version: "1/5".into(),
        }];
        let output = formatter.format_annotation(&entries);
        assert!(output.contains("def456"));
        assert!(output.contains("dev2"));
    }

    #[test]
    fn format_projects_output() {
        let formatter = ResultFormatter::default();
        let output = formatter.format_projects(&["proj-a".into(), "proj-b".into()]);
        assert!(output.contains("Found 2 project(s)"));
        assert!(output.contains("- proj-a"));
        assert!(output.contains("- proj-b"));
    }

    #[test]
    fn format_suggestions_output() {
        let formatter = ResultFormatter::default();
        let suggestions = vec![Suggestion {
            phrase: "funcName".into(),
            projects: vec![],
            score: Some(100),
        }];
        let output = formatter.format_suggestions(&suggestions);
        assert!(output.contains("funcName"));
        assert!(output.contains("score: 100"));
    }
}
