// View module — viewport tools, navigation, palettes, interface.

mod cascade;
mod file_tabs;
mod layout_tabs;
pub mod limits;
mod pan;
pub mod plot_window;
pub mod quick_print;
mod properties_palette;
mod sheetset;
mod tile_horiz;
mod tile_vert;
mod tool_palettes;
mod view_top;
mod vports_config;
mod vports_join;
mod vports_named;
mod vports_restore;
mod zoom_ext;
mod zoom_in;
mod zoom_out;
pub mod zoom_window;

use crate::modules::{CadModule, IconKind, ModuleEvent, RibbonGroup, RibbonItem, ToolDef};

pub struct ViewModule;

impl CadModule for ViewModule {
    fn id(&self) -> &'static str {
        "view"
    }
    fn title(&self) -> &'static str {
        "View"
    }

    fn ribbon_groups(&self) -> &[RibbonGroup] {
        static GROUPS: std::sync::OnceLock<Vec<RibbonGroup>> = std::sync::OnceLock::new();
        GROUPS.get_or_init(|| {
            vec![
                // ── Navigate ─────────────────────────────────────────────────────
                RibbonGroup {
                    title: "Navigate",
                    tools: vec![
                        RibbonItem::LargeTool(zoom_ext::tool()),
                        RibbonItem::Tool(zoom_window::tool()),
                        RibbonItem::Tool(zoom_in::tool()),
                        RibbonItem::Tool(zoom_out::tool()),
                        RibbonItem::Tool(pan::tool()),
                    ],
                },
                // ── Model Viewports ───────────────────────────────────────────────
                RibbonGroup {
                    title: "Model Viewports",
                    tools: vec![
                        RibbonItem::LargeTool(vports_config::tool()),
                        RibbonItem::Tool(vports_named::tool()),
                        RibbonItem::Tool(vports_join::tool()),
                        RibbonItem::Tool(vports_restore::tool()),
                    ],
                },
                // ── Preset Views ──────────────────────────────────────────────────
                RibbonGroup {
                    title: "Preset",
                    tools: vec![
                        RibbonItem::Tool(view_top::tool()),
                    ],
                },
                // ── Palettes ──────────────────────────────────────────────────────
                RibbonGroup {
                    title: "Palettes",
                    tools: vec![
                        RibbonItem::LargeTool(tool_palettes::tool()),
                        RibbonItem::LargeTool(properties_palette::tool()),
                        RibbonItem::LargeTool(sheetset::tool()),
                    ],
                },
                // ── Interface ─────────────────────────────────────────────────────
                RibbonGroup {
                    title: "Interface",
                    tools: vec![
                        RibbonItem::LargeTool(file_tabs::tool()),
                        RibbonItem::LargeTool(layout_tabs::tool()),
                        RibbonItem::Tool(tile_horiz::tool()),
                        RibbonItem::Tool(tile_vert::tool()),
                        RibbonItem::Tool(cascade::tool()),
                    ],
                },
                // ── Plot ──────────────────────────────────────────────────────────
                RibbonGroup {
                    title: "Plot",
                    tools: vec![RibbonItem::Tool(ToolDef {
                        id: "PAGESETUP",
                        label: "Page Setup",
                        icon: IconKind::Svg(include_bytes!("../../../assets/icons/pagesetup.svg")),
                        event: ModuleEvent::Command("PAGESETUP".to_string()),
                    })],
                },
            ]
        })
    }
}
