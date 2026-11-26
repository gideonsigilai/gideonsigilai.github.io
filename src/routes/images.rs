pub async fn creator() -> Response {
    (
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        include_bytes!("../../images/creator.png").as_slice(),
    )
        .into_response()
}
