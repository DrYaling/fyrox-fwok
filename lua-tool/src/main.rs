//! Offline metadata and binding scaffold generator.
//!
//! This tool intentionally runs outside the game. It reads Rust source files,
//! emits an auditable catalog and generates only registration metadata. Runtime
//! bindings still require approval and either a generated wrapper template or a
//! handwritten implementation in lua-plugin.

use quote::ToTokens;
use serde::Serialize;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Serialize)]
struct Catalog {
    schema: u32,
    source: String,
    types: Vec<TypeEntry>,
    functions: Vec<FunctionEntry>,
    methods: Vec<MethodEntry>,
    components: Vec<ComponentEntry>,
}

#[derive(Debug, Serialize)]
struct TypeEntry {
    name: String,
    kind: String,
    fields: Vec<String>,
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct FunctionEntry {
    name: String,
    signature: String,
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct MethodEntry {
    owner: String,
    name: String,
    signature: String,
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct UnsupportedEntry {
    source: String,
    item: String,
    reason: String,
}

#[derive(Debug, Clone, Serialize)]
struct ComponentEntry {
    name: String,
    lua_namespace: String,
    source_type: String,
    category: String,
    methods: Vec<String>,
    status: &'static str,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse(env::args().skip(1))?;
    fs::create_dir_all(&args.output)?;
    let mut files = Vec::new();
    collect_rs(&args.input, &mut files)?;
    if files.is_empty() {
        return Err(format!("no Rust source files found under {}", args.input.display()).into());
    }

    let mut catalog = Catalog {
        schema: 1,
        source: args.input.display().to_string(),
        types: Vec::new(),
        functions: Vec::new(),
        methods: Vec::new(),
        components: match args.profile.as_str() {
            "common" => common_components(),
            "none" => Vec::new(),
            profile => return Err(format!("unknown component profile: {profile}").into()),
        },
    };
    let mut unsupported = Vec::new();
    for file in files {
        parse_file(&file, &mut catalog, &mut unsupported)?;
    }

    let json = serde_json::to_vec_pretty(&catalog)?;
    fs::write(args.output.join("catalog.json"), json)?;
    fs::write(
        args.output.join("unsupported.json"),
        serde_json::to_vec_pretty(&unsupported)?,
    )?;
    write_generated(&args.output.join("generated.rs"), &catalog)?;
    if let Some(runtime_output) = args.runtime_output.as_deref() {
        write_generated(runtime_output, &catalog)?;
    }
    fs::write(
        args.output.join("api-diff.md"),
        format!(
            "# Lua API Diff\n\nGenerated schema: {}\n\nTypes: {}\nFunctions: {}\nMethods: {}\nComponents: {}\nUnsupported items: {}\n",
            catalog.schema,
            catalog.types.len(),
            catalog.functions.len(),
            catalog.methods.len(),
            catalog.components.len(),
            unsupported.len()
        ),
    )?;
    Ok(())
}

struct Args {
    input: PathBuf,
    output: PathBuf,
    runtime_output: Option<PathBuf>,
    profile: String,
}

impl Args {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut input = None;
        let mut output = PathBuf::from("target/lua-bindings");
        let mut runtime_output = None;
        let mut profile = "common".to_owned();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--input" => input = args.next().map(PathBuf::from),
                "--output" => output = args.next().ok_or("--output requires a directory")?.into(),
                "--runtime-output" => {
                    runtime_output = Some(
                        args.next()
                            .ok_or("--runtime-output requires a Rust file")?
                            .into(),
                    )
                }
                "--profile" => profile = args.next().ok_or("--profile requires a name")?,
                "--help" | "-h" => {
                    println!("lua-tool --input <rust-file-or-dir> [--output <dir>]");
                    std::process::exit(0);
                }
                unknown => return Err(format!("unknown argument: {unknown}").into()),
            }
        }
        Ok(Self {
            input: input.ok_or("--input is required")?,
            output,
            runtime_output,
            profile,
        })
    }
}

fn collect_rs(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    if path.is_file() {
        if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path.to_owned());
        }
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let child = entry.path();
        if child.is_dir() {
            collect_rs(&child, files)?;
        } else if child.extension().is_some_and(|ext| ext == "rs") {
            files.push(child);
        }
    }
    files.sort();
    Ok(())
}

fn parse_file(
    path: &Path,
    catalog: &mut Catalog,
    unsupported: &mut Vec<UnsupportedEntry>,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(path)?;
    let file = syn::parse_file(&source)?;
    for item in file.items {
        match item {
            syn::Item::Struct(item) if is_public(&item.vis) => catalog.types.push(TypeEntry {
                name: item.ident.to_string(),
                kind: "struct".into(),
                fields: named_fields(&item.fields),
                status: "CatalogOnly",
            }),
            syn::Item::Enum(item) if is_public(&item.vis) => catalog.types.push(TypeEntry {
                name: item.ident.to_string(),
                kind: "enum".into(),
                fields: enum_variants(&item.variants),
                status: "CatalogOnly",
            }),
            syn::Item::Fn(item) if is_public(&item.vis) => catalog.functions.push(FunctionEntry {
                name: item.sig.ident.to_string(),
                signature: quote_signature(&item.sig),
                status: "CatalogOnly",
            }),
            syn::Item::Impl(item) => {
                let owner = item.self_ty.to_token_stream().to_string();
                for method in item.items {
                    if let syn::ImplItem::Fn(method) = method {
                        if is_public(&method.vis) {
                            catalog.methods.push(MethodEntry {
                                owner: owner.clone(),
                                name: method.sig.ident.to_string(),
                                signature: quote_signature(&method.sig),
                                status: "CatalogOnly",
                            });
                        }
                    }
                }
            }
            syn::Item::Trait(item) => unsupported.push(UnsupportedEntry {
                source: path.display().to_string(),
                item: item.ident.to_string(),
                reason: "traits require a handwritten adapter or explicit generator support".into(),
            }),
            syn::Item::Union(item) => unsupported.push(UnsupportedEntry {
                source: path.display().to_string(),
                item: item.ident.to_string(),
                reason: "unions are not safe to expose automatically".into(),
            }),
            _ => {}
        }
    }
    Ok(())
}

fn is_public(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}

fn named_fields(fields: &syn::Fields) -> Vec<String> {
    fields
        .iter()
        .filter_map(|field| field.ident.as_ref().map(ToString::to_string))
        .collect()
}

fn enum_variants(
    variants: &syn::punctuated::Punctuated<syn::Variant, syn::Token![,]>,
) -> Vec<String> {
    variants
        .iter()
        .map(|variant| variant.ident.to_string())
        .collect()
}

fn quote_signature<T: quote::ToTokens>(value: &T) -> String {
    value.to_token_stream().to_string()
}

fn common_components() -> Vec<ComponentEntry> {
    vec![
        ComponentEntry {
            name: "Widget".into(),
            lua_namespace: "ui.widget".into(),
            source_type: "fyrox::gui::widget::Widget".into(),
            category: "ui".into(),
            methods: [
                "set_text",
                "append",
                "set_visible",
                "set_enabled",
                "set_width",
                "set_height",
                "set_position",
                "text",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
            status: "Implemented",
        },
        ComponentEntry {
            name: "Text".into(),
            lua_namespace: "ui.text".into(),
            source_type: "fyrox::gui::text::Text".into(),
            category: "ui".into(),
            methods: [
                "set_text",
                "append",
                "set_visible",
                "set_enabled",
                "set_position",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
            status: "Implemented",
        },
        ComponentEntry {
            name: "TextBox".into(),
            lua_namespace: "ui.text_box".into(),
            source_type: "fyrox::gui::text_box::TextBox".into(),
            category: "ui".into(),
            methods: [
                "set_text",
                "text",
                "set_visible",
                "set_enabled",
                "set_position",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
            status: "Implemented",
        },
        ComponentEntry {
            name: "Button".into(),
            lua_namespace: "ui.button".into(),
            source_type: "fyrox::gui::button::Button".into(),
            category: "ui".into(),
            methods: [
                "set_text",
                "set_visible",
                "set_enabled",
                "set_position",
                "on_click",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
            status: "Implemented",
        },
        ComponentEntry {
            name: "Node".into(),
            lua_namespace: "scene.node".into(),
            source_type: "fyrox::scene::node::Node".into(),
            category: "3d".into(),
            methods: [
                "set_position",
                "set_rotation_z",
                "set_rotation",
                "set_scale",
                "set_enabled",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
            status: "Implemented",
        },
        ComponentEntry {
            name: "Spatial".into(),
            lua_namespace: "scene.spatial".into(),
            source_type: "fyrox::scene::base::Base".into(),
            category: "3d".into(),
            methods: [
                "set_position",
                "set_rotation_z",
                "set_rotation",
                "set_scale",
                "set_enabled",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
            status: "Implemented",
        },
        ComponentEntry {
            name: "Mesh".into(),
            lua_namespace: "scene.mesh".into(),
            source_type: "fyrox::scene::mesh::Mesh".into(),
            category: "3d".into(),
            methods: ["set_position", "set_rotation", "set_scale", "set_enabled"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            status: "Implemented",
        },
        ComponentEntry {
            name: "Camera".into(),
            lua_namespace: "scene.camera".into(),
            source_type: "fyrox::scene::camera::Camera".into(),
            category: "3d".into(),
            methods: ["set_position", "set_rotation", "set_enabled"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            status: "Implemented",
        },
    ]
}

fn write_generated(path: &Path, catalog: &Catalog) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let mut output = String::from(
        "// @generated by lua-tool; review before enabling.\n\nuse mlua::Lua;\nuse super::{BindingCategory, BindingMethod, BindingRegistry, BindingStatus, BindingType};\n\npub const GENERATED_CATALOG_SCHEMA: u32 = 1;\n\npub fn register_generated_bindings(_lua: &Lua, registry: &mut BindingRegistry) -> mlua::Result<()> {\n    let _schema = GENERATED_CATALOG_SCHEMA;\n",
    );
    for ty in &catalog.types {
        let methods = catalog
            .methods
            .iter()
            .filter(|method| method.owner == ty.name)
            .map(|method| {
                format!(
                    "BindingMethod {{ name: {:?}, signature: {:?}, description: \"Generated public impl method\" }},",
                    method.name, method.signature
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        output.push_str(&format!(
            "    if registry.get({name:?}).is_none() {{\n        registry.register_type(BindingType {{ name: {name:?}, module: \"generated\", category: BindingCategory::Core, description: \"Generated catalog entry\", status: BindingStatus::CatalogOnly, methods: vec![{methods}] }})?;\n    }}\n",
            name = ty.name,
            methods = methods
        ));
    }
    for component in &catalog.components {
        let methods = component
            .methods
            .iter()
            .map(|method| {
                format!(
                    "BindingMethod {{ name: {:?}, signature: \"generic userdata method\", description: \"Generated common component binding\" }},",
                    method
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        let binding_name = component.lua_namespace.clone();
        output.push_str(&format!(
            "    if registry.get({name:?}).is_none() {{\n        registry.register_type(BindingType {{ name: {name:?}, module: {module:?}, category: BindingCategory::{category}, description: \"Generated common Fyrox component alias\", status: BindingStatus::Implemented, methods: vec![{methods}] }})?;\n    }}\n",
            name = binding_name,
            module = component.source_type,
            category = if component.category == "ui" { "Ui" } else { "Scene3D" },
            methods = methods
        ));
    }
    output.push_str("    Ok(())\n}\n");
    output.push_str(
        "\npub fn register_generated_component_aliases(lua: &Lua) -> mlua::Result<()> {\n",
    );
    output.push_str("    let ui: Option<mlua::Table> = lua.globals().get(\"ui\")?;\n    let scene: Option<mlua::Table> = lua.globals().get(\"scene\")?;\n");
    for component in &catalog.components {
        let table = if component.category == "ui" {
            "ui"
        } else {
            "scene"
        };
        let short_name = component
            .lua_namespace
            .rsplit('.')
            .next()
            .unwrap_or(component.name.as_str());
        output.push_str(&format!(
            "    if let Some(table) = {table}.as_ref() {{ table.set({name:?}, table.get::<mlua::Function>(\"find\")?)?; }}\n",
            table = table,
            name = short_name
        ));
    }
    output.push_str("    Ok(())\n}\n");
    fs::write(path, output)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_public_types_methods_and_unsupported_items() {
        let path = env::temp_dir().join(format!("lua-tool-{}.rs", std::process::id()));
        fs::write(
            &path,
            "pub struct Sample { pub value: u32 } impl Sample { pub fn get(&self) -> u32 { self.value } } pub trait External {}",
        )
        .unwrap();
        let mut catalog = Catalog {
            schema: 1,
            source: path.display().to_string(),
            types: Vec::new(),
            functions: Vec::new(),
            methods: Vec::new(),
            components: Vec::new(),
        };
        let mut unsupported = Vec::new();
        parse_file(&path, &mut catalog, &mut unsupported).unwrap();
        assert_eq!(catalog.types[0].name, "Sample");
        assert_eq!(catalog.methods[0].name, "get");
        assert_eq!(unsupported[0].item, "External");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn generated_registration_is_deterministic() {
        let output = env::temp_dir().join(format!("lua-tool-generated-{}.rs", std::process::id()));
        let catalog = Catalog {
            schema: 1,
            source: "test".into(),
            types: vec![TypeEntry {
                name: "Sample".into(),
                kind: "struct".into(),
                fields: vec!["value".into()],
                status: "CatalogOnly",
            }],
            functions: Vec::new(),
            methods: vec![MethodEntry {
                owner: "Sample".into(),
                name: "get".into(),
                signature: "fn get(&self) -> u32".into(),
                status: "CatalogOnly",
            }],
            components: common_components(),
        };
        write_generated(&output, &catalog).unwrap();
        let generated = fs::read_to_string(&output).unwrap();
        assert!(generated.contains("registry.get(\"Sample\")"));
        assert!(generated.contains("register_generated_bindings"));
        assert!(generated.contains("name: \"get\""));
        assert!(generated.contains("register_generated_component_aliases"));
        assert!(generated.contains("table.set(\"text\""));
        let _ = fs::remove_file(output);
    }
}
