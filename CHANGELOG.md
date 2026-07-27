# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project aims to
follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- 10 new MCP tools for remaining public (non-Bearer) OpenGrok API endpoints:
  - `list_groups` / `get_group_projects` — project group navigation
  - `list_project_files` / `list_project_repos` / `get_project_property` — project metadata
  - `get_repo_property` — repository property lookup
  - `get_suggest_config` — suggester configuration
  - `health_check` / `get_index_time` / `get_opengrok_version` — system info
- Total tools: 15 → 25 with 100% test coverage on new code (158 tests).
- `SuggestConfig` domain model for `/suggest/config` response.
- Bilingual documentation (EN + RU) in `docs/en/` and `docs/ru/`: overview, installation,
  usage (config reference + all 25 MCP tools), architecture (workspace layout, crate
  responsibilities, data flow, design decisions), and development guide (contributing,
  testing, CI, adding new tools).
- Bilingual README.md with English and Russian sections.
