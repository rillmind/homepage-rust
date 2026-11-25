use axum::{
  Router,
  http::{HeaderName, HeaderValue, header},
};
use tower::ServiceBuilder;
use tower_http::{
  services::{ServeDir, ServeFile},
  set_header::SetResponseHeaderLayer,
};

fn set(name: HeaderName, value: &'static str) -> SetResponseHeaderLayer<HeaderValue> {
  SetResponseHeaderLayer::overriding(name, HeaderValue::from_static(value))
}

/// Arquivos do front, já comprimidos no build. Rota desconhecida abre o index.
pub fn routes(dir: String) -> Router {
  let files = |path: &str| ServeDir::new(format!("{dir}{path}")).precompressed_gzip();
  let index = ServeFile::new(format!("{dir}/index.html")).precompressed_gzip();

  // Os nomes em /assets têm o hash do conteúdo, então podem ficar em cache para sempre.
  let assets = ServiceBuilder::new()
    .layer(set(
      header::CACHE_CONTROL,
      "public, max-age=31536000, immutable",
    ))
    .service(files("/assets"));
  let pages = ServiceBuilder::new()
    .layer(set(header::CACHE_CONTROL, "no-cache"))
    .service(files("").fallback(index));

  Router::new()
    .nest_service("/assets", assets)
    .fallback_service(pages)
    .layer(set(header::X_FRAME_OPTIONS, "SAMEORIGIN"))
    .layer(set(header::X_CONTENT_TYPE_OPTIONS, "nosniff"))
    .layer(set(header::REFERRER_POLICY, "no-referrer"))
}
