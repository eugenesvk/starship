use super::{Context, Module, RootModuleConfig};

use crate::configs::lua::LuaConfig;
use crate::formatter::StringFormatter;
use crate::utils;

use regex::Regex;
const LUA_VERSION_PATERN: &str = "(?P<version>[\\d\\.]+[a-z\\-]*[1-9]*)[^\\s]*";

/// Creates a module with the current Lua version
///
/// Will display the Lua version if any of the following criteria are met:
///     - Current directory contains a `.lua-version` file
///     - Current directory contains a `lua` directory
///     - Current directory contains a file with the `.lua` extension
pub fn module<'a>(context: &'a Context) -> Option<Module<'a>> {
    let is_lua_project = context
        .try_begin_scan()?
        .set_files(&[".lua-version"])
        .set_folders(&["lua"])
        .set_extensions(&["lua"])
        .is_match();

    if !is_lua_project {
        return None;
    }

    let mut module = context.new_module("lua");
    let config = LuaConfig::try_load(module.config);
    let parsed = StringFormatter::new(config.format).and_then(|formatter| {
        formatter
            .map_meta(|var, _| match var {
                "symbol" => Some(config.symbol),
                _ => None,
            })
            .map_style(|variable| match variable {
                "style" => Some(Ok(config.style)),
                _ => None,
            })
            .map(|variable| match variable {
                "version" => {
                    let lua_version = format_lua_version(&get_lua_version(&config.lua_binary)?)?;
                    Some(Ok(lua_version))
                }
                _ => None,
            })
            .parse(None)
    });

    module.set_segments(match parsed {
        Ok(segments) => segments,
        Err(error) => {
            log::warn!("Error in module `lua`:\n{}", error);
            return None;
        }
    });

    Some(module)
}

fn get_lua_version(lua_binary: &str) -> Option<String> {
    match utils::exec_cmd(lua_binary, &["-v"]) {
        Some(output) => {
            if output.stdout.is_empty() {
                Some(output.stderr)
            } else {
                Some(output.stdout)
            }
        }
        None => None,
    }
}

fn format_lua_version(lua_stdout: &str) -> Option<String> {
    // lua -v output looks like this:
    // Lua 5.4.0  Copyright (C) 1994-2020 Lua.org, PUC-Rio

    // luajit -v output looks like this:
    // LuaJIT 2.0.5 -- Copyright (C) 2005-2017 Mike Pall. http://luajit.org/
    let re = Regex::new(LUA_VERSION_PATERN).ok()?;
    let captures = re.captures(lua_stdout)?;
    let version = &captures["version"];
    Some(version.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::ModuleRenderer;
    use ansi_term::Color;
    use std::fs::{self, File};
    use std::io;

    #[test]
    fn folder_without_lua_files() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        let actual = ModuleRenderer::new("lua").path(dir.path()).collect();
        let expected = None;
        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn folder_with_lua_file() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        File::create(dir.path().join("main.lua"))?.sync_all()?;
        let actual = ModuleRenderer::new("lua").path(dir.path()).collect();
        let expected = Some(format!("via {}", Color::Blue.bold().paint("🌙 v5.4.0 ")));
        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn folder_with_lua_version() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        File::create(dir.path().join(".lua-version"))?.sync_all()?;

        let actual = ModuleRenderer::new("lua").path(dir.path()).collect();
        let expected = Some(format!("via {}", Color::Blue.bold().paint("🌙 v5.4.0 ")));
        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn folder_with_lua_folder() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        let lua_dir = dir.path().join("lua");
        fs::create_dir_all(&lua_dir)?;

        let actual = ModuleRenderer::new("lua").path(dir.path()).collect();
        let expected = Some(format!("via {}", Color::Blue.bold().paint("🌙 v5.4.0 ")));
        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn lua_binary_is_luajit() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        File::create(dir.path().join("main.lua"))?.sync_all()?;

        let config = toml::toml! {
             [lua]
             lua_binary = "luajit"
        };

        let actual = ModuleRenderer::new("lua")
            .path(dir.path())
            .config(config)
            .collect();

        let expected = Some(format!("via {}", Color::Blue.bold().paint("🌙 v2.0.5 ")));
        assert_eq!(expected, actual);
        dir.close()
    }

    #[test]
    fn test_format_lua_version() {
        let lua_input = "Lua 5.4.0  Copyright (C) 1994-2020 Lua.org, PUC-Rio";
        assert_eq!(format_lua_version(lua_input), Some("5.4.0".to_string()));

        let luajit_input =
            "LuaJIT 2.1.0-beta3 -- Copyright (C) 2005-2017 Mike Pall. http://luajit.org/";
        assert_eq!(
            format_lua_version(luajit_input),
            Some("2.1.0-beta3".to_string())
        );
    }
}
