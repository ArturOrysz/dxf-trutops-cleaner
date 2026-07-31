fn main() {
    #[cfg(windows)]
    {
        embed_resource::compile("assets/app.rc", embed_resource::NONE)
            .manifest_optional()
            .ok();
    }
}
