use zed_extension_api as zed;

struct HurlExtension;

impl zed::Extension for HurlExtension {
    fn new() -> Self {
        HurlExtension
    }

    fn language_server_command(
        &mut self,
        _id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command, String> {
        let command = worktree
            .which("hurl-lsp")
            .ok_or_else(|| "hurl-lsp not found in PATH".to_string())?;

        Ok(zed::Command {
            command,
            args: vec![],
            env: vec![],
        })
    }
}

zed::register_extension!(HurlExtension);
