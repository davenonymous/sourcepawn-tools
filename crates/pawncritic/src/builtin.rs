// Copyright 2026 davenonymous

//! The Lua policies that ship with pawncritic and the libraries they share,
//! compiled into the binary.

/// Every built-in policy: its path below `policies/` and its code.
pub const SOURCES: &[(&str, &str)] = &[
    ("Bugs/RequireInclusiveMaxClients.lua", include_str!("../policies/Bugs/RequireInclusiveMaxClients.lua")),
    ("Bugs/RequireUserIdInTimers.lua", include_str!("../policies/Bugs/RequireUserIdInTimers.lua")),
    ("CodeLayout/ProhibitLongLines.lua", include_str!("../policies/CodeLayout/ProhibitLongLines.lua")),
    ("CodeLayout/ProhibitMixedIndentation.lua", include_str!("../policies/CodeLayout/ProhibitMixedIndentation.lua")),
    (
        "CodeLayout/ProhibitTrailingWhitespace.lua",
        include_str!("../policies/CodeLayout/ProhibitTrailingWhitespace.lua"),
    ),
    ("CodeLayout/RequireBracedBlocks.lua", include_str!("../policies/CodeLayout/RequireBracedBlocks.lua")),
    ("CodeLayout/RequireSameLineBraces.lua", include_str!("../policies/CodeLayout/RequireSameLineBraces.lua")),
    ("Complexity/ProhibitLongFiles.lua", include_str!("../policies/Complexity/ProhibitLongFiles.lua")),
    ("Complexity/ProhibitLongFunctions.lua", include_str!("../policies/Complexity/ProhibitLongFunctions.lua")),
    ("Malicious/ProhibitClientSlowhack.lua", include_str!("../policies/Malicious/ProhibitClientSlowhack.lua")),
    ("Malicious/ProhibitDangerousDownloads.lua", include_str!("../policies/Malicious/ProhibitDangerousDownloads.lua")),
    ("Malicious/ProhibitObfuscatedStrings.lua", include_str!("../policies/Malicious/ProhibitObfuscatedStrings.lua")),
    ("Malicious/ProhibitPasswordConVars.lua", include_str!("../policies/Malicious/ProhibitPasswordConVars.lua")),
    ("Malicious/ProhibitSteamIdBackdoors.lua", include_str!("../policies/Malicious/ProhibitSteamIdBackdoors.lua")),
    ("Malicious/ProhibitVguiCommandBypass.lua", include_str!("../policies/Malicious/ProhibitVguiCommandBypass.lua")),
    ("Modern/ProhibitCloseHandle.lua", include_str!("../policies/Modern/ProhibitCloseHandle.lua")),
    ("Modern/ProhibitDeprecatedFunctions.lua", include_str!("../policies/Modern/ProhibitDeprecatedFunctions.lua")),
    ("Modern/ProhibitFunctagAndFuncenum.lua", include_str!("../policies/Modern/ProhibitFunctagAndFuncenum.lua")),
    ("Modern/ProhibitLegacyApiFunctions.lua", include_str!("../policies/Modern/ProhibitLegacyApiFunctions.lua")),
    ("Modern/ProhibitOldDeclarations.lua", include_str!("../policies/Modern/ProhibitOldDeclarations.lua")),
    ("Modern/ProhibitOldTypeTags.lua", include_str!("../policies/Modern/ProhibitOldTypeTags.lua")),
    ("Modern/ProhibitTagCasts.lua", include_str!("../policies/Modern/ProhibitTagCasts.lua")),
    ("Modern/RequireNewDecls.lua", include_str!("../policies/Modern/RequireNewDecls.lua")),
    ("Modern/RequireSemicolons.lua", include_str!("../policies/Modern/RequireSemicolons.lua")),
    (
        "NamingConventions/RequireGlobalPrefix.lua",
        include_str!("../policies/NamingConventions/RequireGlobalPrefix.lua"),
    ),
    ("Philosophy/ProhibitBareHandle.lua", include_str!("../policies/Philosophy/ProhibitBareHandle.lua")),
    ("Philosophy/ProhibitDeepNesting.lua", include_str!("../policies/Philosophy/ProhibitDeepNesting.lua")),
    ("Philosophy/ProhibitElseAfterReturn.lua", include_str!("../policies/Philosophy/ProhibitElseAfterReturn.lua")),
    ("Philosophy/ProhibitEmptyBlocks.lua", include_str!("../policies/Philosophy/ProhibitEmptyBlocks.lua")),
    (
        "Philosophy/ProhibitHiddenGlobalMutation.lua",
        include_str!("../policies/Philosophy/ProhibitHiddenGlobalMutation.lua"),
    ),
    ("Philosophy/ProhibitMeaninglessNames.lua", include_str!("../policies/Philosophy/ProhibitMeaninglessNames.lua")),
    (
        "Philosophy/RequireBooleanPredicateNames.lua",
        include_str!("../policies/Philosophy/RequireBooleanPredicateNames.lua"),
    ),
    ("Philosophy/RequireEarlyExit.lua", include_str!("../policies/Philosophy/RequireEarlyExit.lua")),
    ("Security/ProhibitAbsolutePaths.lua", include_str!("../policies/Security/ProhibitAbsolutePaths.lua")),
    ("Security/ProhibitCodeFileWrites.lua", include_str!("../policies/Security/ProhibitCodeFileWrites.lua")),
    ("Security/ProhibitCommandInjection.lua", include_str!("../policies/Security/ProhibitCommandInjection.lua")),
    ("Security/ProhibitHardcodedEndpoints.lua", include_str!("../policies/Security/ProhibitHardcodedEndpoints.lua")),
    ("Security/ProhibitNonLiteralFormat.lua", include_str!("../policies/Security/ProhibitNonLiteralFormat.lua")),
    ("Security/ProhibitPathTraversal.lua", include_str!("../policies/Security/ProhibitPathTraversal.lua")),
    ("Security/ProhibitSecretFileAccess.lua", include_str!("../policies/Security/ProhibitSecretFileAccess.lua")),
    (
        "Security/ProhibitServerControlCommands.lua",
        include_str!("../policies/Security/ProhibitServerControlCommands.lua"),
    ),
    ("Security/ProhibitShellExecution.lua", include_str!("../policies/Security/ProhibitShellExecution.lua")),
    ("Security/ProhibitSqlInjection.lua", include_str!("../policies/Security/ProhibitSqlInjection.lua")),
    ("Variables/RequireLateDeclarations.lua", include_str!("../policies/Variables/RequireLateDeclarations.lua")),
    ("Variables/RequireNarrowDeclarations.lua", include_str!("../policies/Variables/RequireNarrowDeclarations.lua")),
];

/// Every built-in library: the name `pawn.library` knows it by, and its
/// code, in `policies/lib/<name>.lua`.
pub const LIBRARIES: &[(&str, &str)] = &[
    ("calls", include_str!("../policies/lib/calls.lua")),
    ("declarations", include_str!("../policies/lib/declarations.lua")),
    ("functions", include_str!("../policies/lib/functions.lua")),
    ("lines", include_str!("../policies/lib/lines.lua")),
    ("paths", include_str!("../policies/lib/paths.lua")),
    ("tags", include_str!("../policies/lib/tags.lua")),
    ("taint", include_str!("../policies/lib/taint.lua")),
];

/// The directory below `policies/` that holds the libraries.
pub const LIBRARY_DIRECTORY: &str = "lib";

#[cfg(test)]
mod tests {
    use super::*;

    /// Every file in `policies/` is listed, the policies in [`SOURCES`] and
    /// the libraries in `lib/` in [`LIBRARIES`], so none is forgotten.
    #[test]
    fn every_policy_file_is_built_in() {
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("policies");
        let mut on_disk = Vec::new();
        for category in std::fs::read_dir(&directory).unwrap() {
            let category = category.unwrap().path();
            for file in std::fs::read_dir(&category).unwrap() {
                let file = file.unwrap().path();
                let relative = file.strip_prefix(&directory).unwrap().to_string_lossy().replace('\\', "/");
                on_disk.push(relative);
            }
        }
        on_disk.sort();
        let library_prefix = format!("{LIBRARY_DIRECTORY}/");
        let (libraries, policies): (Vec<String>, Vec<String>) =
            on_disk.into_iter().partition(|file| file.starts_with(&library_prefix));
        let listed: Vec<String> = SOURCES.iter().map(|(file, _)| file.to_string()).collect();
        assert_eq!(listed, policies);
        let listed_libraries: Vec<String> =
            LIBRARIES.iter().map(|(name, _)| format!("{LIBRARY_DIRECTORY}/{name}.lua")).collect();
        assert_eq!(listed_libraries, libraries);
    }

    /// Every library loads into a policy's state, returns a table and is
    /// documented with each of its members in `docs/writing-policies.md`.
    #[test]
    fn every_library_loads_and_is_documented() {
        let documentation = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/writing-policies.md");
        let text = std::fs::read_to_string(documentation).expect("docs/writing-policies.md exists");
        for (name, _) in LIBRARIES {
            let deadline = crate::lua::sandbox::Deadline::default();
            let lua = crate::lua::sandbox::new_state("Test::Library", Default::default(), &deadline).unwrap();
            crate::lua::api::install_helpers(&lua).unwrap();
            let library: mlua::Table = lua.load(format!("return pawn.library(\"{name}\")")).eval().unwrap();
            assert!(text.contains(&format!("`pawn.library(\"{name}\")`")), "the library {name} is not documented");
            for pair in library.pairs::<String, mlua::Value>() {
                let (member, _) = pair.unwrap();
                assert!(text.contains(&format!("`{name}.{member}")), "{name}.{member} is not documented");
            }
        }
    }
}
