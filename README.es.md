<h1 align="center">🌐 OpenGraph Scraper 🌐</h1>

Scraper rápido y seguro contra SSRF de metadatos OpenGraph, expuesto como API
JSON. Escrito en Rust con [Axum](https://github.com/tokio-rs/axum).

## Características

- **Extracción OpenGraph**: `og:title`, `og:description`, `og:image` (más
  tarjetas `twitter:*`) con respaldos `<title>` y `meta[name=description]`,
  decodificación de entidades HTML.
- **Parseo en streaming**: lee solo `<head>`, corta la conexión en cuanto tiene
  título, descripción e imagen, tope de 512 KB, ventana deslizante de 8 KB
  (memoria O(1), sin regex).
- **Protección SSRF**
  - Solo `http`/`https`, sin credenciales en la URL, puertos limitados a
    80/443.
  - Bloquea `localhost`, rangos privados, loopback, link-local, multicast, CGNAT
    (`100.64/10`), benchmarking (`198.18/15`), reservadas (`240/4`) y rangos
    IPv6 especiales; detecta evasiones con literales IP decimales, octales y hex
    (`0x7f.0.0.1`, `2130706433`) y la `::ffff:127.0.0.1` mapeada.
  - Cada salto de redirect se revalida (hasta 3, seguidos manualmente) y las
    respuestas DNS se fijan con pinning, cerrando la brecha TOCTOU entre
    resolver y conectar.
- **Descarga resiliente**: timeouts de 4s total / 2s de conexión, 3s por chunk
  estancado, un reintento suave ante errores de conexión y `502`/`503`, rotando
  entre tres user agents crawlers de Meta.
- **Errores estrictos**: todo fallo es `application/problem+json` (RFC 9457),
  los problemas de validación devuelven `422` con violaciones por campo.
- **Docs incluidas**: OpenAPI en `/openapi.json`, referencia interactiva en
  `/docs` (Scalar).
- **Logs estructurados**: `tracing` con host, latencia y bytes por request; las
  URLs nunca se registran completas.

## Licencia

MIT — libre para uso personal y comercial. Ver [LICENSE](LICENSE).
