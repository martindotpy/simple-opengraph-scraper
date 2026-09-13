<h1 align="center">🌐 OpenGraph Scraper 🌐</h1>

Fast, SSRF-safe OpenGraph metadata scraper exposed as a JSON API. Written in
Rust with [Axum](https://github.com/tokio-rs/axum).

## Features

- **OpenGraph extraction**: `og:title`, `og:description`, `og:image` (plus
  `twitter:*` cards) with `<title>` and `meta[name=description]` fallbacks, HTML
  entity decoding.
- **Streaming parser**: reads only `<head>`, aborts the connection as soon as
  title, description and image are complete, 512 KB hard cap, 8 KB sliding
  window (O(1) memory, no regex).
- **SSRF protection**
  - Allows only `http`/`https`, no credentials in URL, ports restricted to
    80/443.
  - Blocks `localhost`, private, loopback, link-local, multicast, CGNAT
    (`100.64/10`), benchmarking (`198.18/15`), reserved (`240/4`) and IPv6
    special ranges; catches decimal/octal/hex IP literal evasions (`0x7f.0.0.1`,
    `2130706433`) and mapped `::ffff:127.0.0.1`.
  - Every redirect hop is revalidated (up to 3, followed manually) and DNS
    answers are pinned, closing the resolve - connect TOCTOU gap.
- **Resilient fetching**: 4s total / 2s connect timeouts, 3s per-chunk stall
  timeout, one soft retry on connect errors and `502`/`503`, rotating across
  three Meta crawler user agents.
- **Strict errors**: every failure is `application/problem+json` (RFC 9457),
  validation issues return `422` with per-field violations.
- **Built-in docs**: OpenAPI at `/openapi.json`, interactive reference at
  `/docs` (Scalar).
- **Structured logs**: `tracing` with per-request host, latency and byte counts;
  URLs are never logged whole.

## License

MIT — free for personal and commercial use. See [LICENSE](LICENSE).
