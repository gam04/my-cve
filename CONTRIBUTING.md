# Contribuir

¡Gracias por querer mejorar **Ficha del día**!

## Antes de empezar

- Revisá los [issues](https://github.com/gam04/my-cve/issues) abiertos para evitar trabajo duplicado.
- Para cambios grandes (arquitectura, nuevas fuentes de datos, rediseño), abrí un issue primero para acordar el enfoque.

## Entorno de desarrollo

Requisitos: Node.js ≥ 20, pnpm 11, Rust estable, WebView2 (Windows).

```bash
pnpm install
pnpm app:dev      # Tauri + Vite en caliente
```

Comprobaciones locales (las mismas que exige CI):

```bash
pnpm check
pnpm test
pnpm build
cargo fmt --check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

Para validar layout/UI sin abrir la app: `pnpm build` y luego `pnpm exec vite preview --port 1420 --strictPort` + `node scripts/audit.mjs`.

## Pull requests

- Una rama por cambio, commits claros.
- Incluí pruebas cuando el cambio sea de lógica (ranking, normalización, caché, formato).
- No modifiques `package.json`/`Cargo.toml`/`tauri.conf.json` de versión salvo que se coordine un release.
- Asegurate de que `pnpm check`, `pnpm test`, `cargo fmt`, `cargo clippy -D warnings` y `cargo test` pasen.

## Seguridad

No reportes vulnerabilidades en issues públicos; seguí [SECURITY.md](SECURITY.md).
