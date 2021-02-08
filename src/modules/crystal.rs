use super::{Context, Module, RootModuleConfig};

use crate::configs::crystal::CrystalConfig;
use crate::formatter::StringFormatter;
use crate::utils;

/// Creates a module with the current Crystal version
///
/// Will display the Crystal version if any of the following criteria are met:
///     - Current directory contains a `.cr` file
///     - Current directory contains a `shard.yml` file
pub fn module<'a>(context: &'a Context) -> Option<Module<'a>> {
    let is_crystal_project = context
        .try_begin_scan()?
        .set_files(&["shard.yml"])
        .set_extensions(&["cr"])
        .is_match();

    if !is_crystal_project {
        return None;
    }

    let mut module = context.new_module("crystal");
    let config: CrystalConfig = CrystalConfig::try_load(module.config);

    let parsed = StringFormatter::new(config.format).and_then(|formatter| {
        formatter
            .map_meta(|variable, _| match variable {
                "symbol" => Some(config.symbol),
                _ => None,
            })
            .map_style(|variable| match variable {
                "style" => Some(Ok(config.style)),
                _ => None,
            })
            .map(|variable| match variable {
                "version" => format_crystal_version(
                    utils::exec_cmd("crystal", &["--version"])?.stdout.as_str(),
                )
                .map(Ok),
                _ => None,
            })
            .parse(None)
    });

    module.set_segments(match parsed {
        Ok(segments) => segments,
        Err(error) => {
            log::warn!("Error in module `crystal`:\n{}", error);
            return None;
        }
    });

    Some(module)
}

fn format_crystal_version(crystal_version: &str) -> Option<String> {
    let version = crystal_version
        // split into ["Crystal", "0.35.1", ...]
        .split_whitespace()
        // return "0.35.1"
        .nth(1)?;

    Some(version.to_string())
}

#[cfg(test)]
mod tests {
    use crate::test::ModuleRenderer;
    use ansi_term::Color;
    use std::fs::File;
    use std::io;

    #[test]
    fn folder_without_crystal_files() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        let actual = ModuleRenderer::new("crystal").path(dir.path()).collect();
        let expected = None;
        assert_eq!(expected, actual);

        dir.close()
    }

    #[test]
    fn folder_with_shard_file() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        File::create(dir.path().join("shard.yml"))?.sync_all()?;

        let actual = ModuleRenderer::new("crystal").path(dir.path()).collect();
        let expected = Some(format!("via {}", Color::Red.bold().paint("🔮 v0.35.1 ")));
        assert_eq!(expected, actual);

        dir.close()
    }

    #[test]
    fn folder_with_cr_file() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        File::create(dir.path().join("main.cr"))?.sync_all()?;

        let actual = ModuleRenderer::new("crystal").path(dir.path()).collect();
        let expected = Some(format!("via {}", Color::Red.bold().paint("🔮 v0.35.1 ")));
        assert_eq!(expected, actual);

        dir.close()
    }
}
