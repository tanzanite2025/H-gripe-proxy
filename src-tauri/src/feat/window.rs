pub async fn quit() {
    crate::app::window::quit().await;
}

#[cfg(target_os = "macos")]
pub async fn hide() {
    crate::app::window::hide().await;
}
