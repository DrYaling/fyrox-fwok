//! Offline UI resource assembler for the Lua binding demonstration.
//!
//! This binary is deliberately separate from the runtime plugin. It creates
//! serialized Fyrox controls in `data/unnamed.ui`; Lua only resolves and
//! operates those controls at runtime.

use fyrox::{
    asset::{io::FsResourceIo, manager::ResourceManager},
    core::{
        algebra::Vector2, dyntype::DynTypeConstructorContainer, futures::executor::block_on,
        task::TaskPool,
    },
    graph::SceneGraph,
    gui::{
        border::BorderBuilder,
        canvas::CanvasBuilder,
        check_box::CheckBoxBuilder,
        constructor::new_widget_constructor_container,
        dropdown_list::DropdownListBuilder,
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        popup::PopupBuilder,
        progress_bar::ProgressBarBuilder,
        scroll_panel::ScrollPanelBuilder,
        scroll_viewer::ScrollViewerBuilder,
        text::TextBuilder,
        widget::WidgetBuilder,
        UiNode, UserInterface,
    },
};
use std::{path::Path, sync::Arc};

const UI_PATH: &str = "data/unnamed.ui";
const PANEL_NAME: &str = "lua_component_demo_panel";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let task_pool = Arc::new(TaskPool::new());
    let resource_manager = ResourceManager::new(Arc::new(FsResourceIo), task_pool);
    // The standalone tool does not run Fyrox's Engine bootstrap, so register
    // resource constructors required by serialized UI brush/material data.
    resource_manager
        .state()
        .constructors_container
        .add::<fyrox::material::Material>();
    resource_manager.update_or_load_registry();

    let (mut ui, _) = block_on(UserInterface::load_from_file(
        UI_PATH,
        Arc::new(new_widget_constructor_container()),
        Arc::new(DynTypeConstructorContainer::default()),
        resource_manager,
    ))?;

    if let Some((panel, _)) = ui.find_by_name_from_root(PANEL_NAME) {
        // This is a generated demo subtree, so replacing only that subtree is
        // deterministic and leaves the user's HUD untouched.
        ui.remove_node(panel);
    }

    let panel = build_demo_panel(&mut ui);
    ui.link_nodes(panel, ui.root(), false);
    ui.save(Path::new(UI_PATH))?;
    println!("Serialized Lua UI component demo into {UI_PATH}");
    Ok(())
}

fn named_widget(name: &str, x: f32, y: f32, width: f32, height: f32) -> WidgetBuilder {
    WidgetBuilder::new()
        .with_name(name)
        .with_desired_position(Vector2::new(x, y))
        .with_width(width)
        .with_height(height)
}

fn build_demo_panel(ui: &mut UserInterface) -> fyrox::core::pool::Handle<UiNode> {
    let ctx = &mut ui.build_ctx();
    let selector_items = vec![
        TextBuilder::new(WidgetBuilder::new())
            .with_text("One")
            .build(ctx)
            .to_base(),
        TextBuilder::new(WidgetBuilder::new())
            .with_text("Two")
            .build(ctx)
            .to_base(),
    ];
    let scroll_content =
        TextBuilder::new(WidgetBuilder::new().with_width(240.0).with_height(180.0))
            .with_text("Lua scroll viewer content\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6")
            .build(ctx);
    let scroll_panel_content =
        TextBuilder::new(WidgetBuilder::new().with_width(180.0).with_height(150.0))
            .with_text("Lua scroll panel content\nLine 2\nLine 3\nLine 4\nLine 5")
            .build(ctx);
    let popup_content = TextBuilder::new(WidgetBuilder::new().with_width(180.0).with_height(32.0))
        .with_text("Lua popup content")
        .build(ctx);
    let grid_child = TextBuilder::new(
        named_widget("demo_grid_child", 0.0, 0.0, 120.0, 22.0)
            .on_row(0)
            .on_column(0),
    )
    .with_text("Grid child")
    .build(ctx);

    let toggle = CheckBoxBuilder::new(named_widget("demo_toggle", 12.0, 34.0, 180.0, 24.0))
        .checked(Some(true))
        .build(ctx);
    let selector = DropdownListBuilder::new(named_widget("demo_selector", 12.0, 66.0, 180.0, 28.0))
        .with_items(selector_items)
        .with_selected(0)
        .build(ctx);
    let progress = ProgressBarBuilder::new(named_widget("demo_progress", 12.0, 104.0, 180.0, 18.0))
        .with_progress(0.35)
        .build(ctx);
    let scroll_panel = ScrollPanelBuilder::new(
        named_widget("demo_scroll_panel", 12.0, 132.0, 180.0, 48.0)
            .with_child(scroll_panel_content),
    )
    .with_vertical_scroll_allowed(true)
    .with_scroll_value(Vector2::new(0.0, 0.0))
    .build(ctx);
    let scroll_viewer = ScrollViewerBuilder::new(named_widget(
        "demo_scroll_viewer",
        204.0,
        34.0,
        210.0,
        112.0,
    ))
    .with_content(scroll_content)
    .with_vertical_scroll_allowed(true)
    .build(ctx);
    let popup = PopupBuilder::new(named_widget("demo_popup", 204.0, 156.0, 180.0, 34.0))
        .stays_open(true)
        .with_content(popup_content)
        .build(ctx);
    let image = ImageBuilder::new(named_widget("demo_image", 396.0, 156.0, 20.0, 20.0)).build(ctx);
    let grid = GridBuilder::new(
        named_widget("demo_grid", 12.0, 190.0, 180.0, 32.0).with_child(grid_child),
    )
    .add_row(Row::stretch())
    .add_column(Column::stretch())
    .build(ctx);
    let canvas =
        CanvasBuilder::new(named_widget("demo_canvas", 204.0, 200.0, 210.0, 24.0)).build(ctx);

    BorderBuilder::new(
        named_widget(PANEL_NAME, 18.0, 390.0, 430.0, 240.0).with_children([
            toggle.to_base(),
            selector.to_base(),
            progress.to_base(),
            scroll_panel.to_base(),
            scroll_viewer.to_base(),
            popup.to_base(),
            image.to_base(),
            grid.to_base(),
            canvas.to_base(),
        ]),
    )
    .build(ctx)
    .to_base()
}
