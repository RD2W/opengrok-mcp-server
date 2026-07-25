// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! Core library for the OpenGrok MCP server.
//!
//! Contains domain models, the [`OpengrokRepository`] trait, the application
//! service layer ([`OpengrokService`]), and infrastructure implementations
//! (HTTP client, TLS, cache, rate-limit, result formatting).

pub mod application;
pub mod domain;
pub mod infrastructure;
