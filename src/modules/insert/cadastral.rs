use crate::modules::{IconKind, ModuleEvent, ToolDef};
pub const ICON: IconKind = IconKind::Svg(include_bytes!("../../../assets/icons/landxml.svg"));
pub fn tool() -> ToolDef {
    ToolDef {
        id: "CADASTRAL",
        label: "Cadastral",
        icon: ICON,
        event: ModuleEvent::Command("CADASTRAL".to_string()),
    }
}
