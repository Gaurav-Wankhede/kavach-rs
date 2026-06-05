//! Component-monolith detector coverage: export counting, escape hatches,
//! directory-agnostic firing, test/non-frontend exemptions.
use crate::gates::pre_write_ts_guard::check_component_oversized;

#[test]
fn should_block_component_oversized_with_multiple_exports() {
    let mut body = "export function Sidebar() { return <div/>; }\n\
                    export function Header() { return <div/>; }\n"
        .to_owned();
    body.push_str(&"// padding\n".repeat(100));
    let result = check_component_oversized("src/components/Layout.tsx", &body);
    assert!(result.is_some());
    let msg = result.unwrap_or_default();
    assert!(msg.contains("COMPONENT_MONOLITH"));
    assert!(msg.contains('2'));
}

#[test]
fn should_not_block_single_export_component() {
    let mut body = "export default function Dashboard() { return <div/>; }\n".to_owned();
    body.push_str(&"// padding\n".repeat(110));
    assert!(check_component_oversized("src/pages/Dashboard.tsx", &body).is_none());
}

#[test]
fn should_not_block_component_under_line_limit() {
    let body = "export function A() {}\nexport function B() {}\n".repeat(3);
    assert!(check_component_oversized("src/components/Small.tsx", &body).is_none());
}

#[test]
fn should_not_block_with_split_escape_hatch() {
    let mut body = "// split: compound component\n".to_owned();
    body.push_str(&"export function A() {}\nexport function B() {}\n".repeat(52));
    assert!(check_component_oversized("src/components/Compound.tsx", &body).is_none());
}

#[test]
fn should_not_block_with_jsx_split_escape_hatch() {
    let mut body = "{/* split: compound component */}\n".to_owned();
    body.push_str(&"export function A() {}\nexport function B() {}\n".repeat(52));
    assert!(check_component_oversized("src/components/Compound.jsx", &body).is_none());
}

#[test]
fn should_not_count_type_or_interface_exports() {
    let mut body = "export type Props = { x: string };\n\
                    export interface Config { y: number; }\n\
                    export function App() { return <div/>; }\n"
        .to_owned();
    body.push_str(&"// padding\n".repeat(100));
    assert!(check_component_oversized("src/App.tsx", &body).is_none());
}

#[test]
fn should_count_uppercase_const_exports_as_components() {
    let mut body = "export const Sidebar = () => <div/>;\n\
                    export const Header = () => <div/>;\n"
        .to_owned();
    body.push_str(&"// padding\n".repeat(100));
    assert!(check_component_oversized("src/Layout.tsx", &body).is_some());
}

#[test]
fn should_not_count_lowercase_const_exports() {
    let mut body = "export const config = { theme: 'dark' };\n\
                    export const utils = { format: () => {} };\n\
                    export function App() { return <div/>; }\n"
        .to_owned();
    body.push_str(&"// padding\n".repeat(100));
    assert!(check_component_oversized("src/App.tsx", &body).is_none());
}

#[test]
fn should_block_in_any_directory() {
    let mut body = "export function A() {}\nexport function B() {}\n".to_owned();
    body.push_str(&"// padding\n".repeat(100));
    assert!(check_component_oversized("src/pages/Home.tsx", &body).is_some());
    assert!(check_component_oversized("src/islands/Nav.tsx", &body).is_some());
    assert!(check_component_oversized("packages/shared/UI.jsx", &body).is_some());
    assert!(check_component_oversized("src/Layout.astro", &body).is_some());
}

#[test]
fn should_skip_test_files_for_component_oversized() {
    let mut body = "export function A() {}\nexport function B() {}\n".to_owned();
    body.push_str(&"// padding\n".repeat(100));
    assert!(check_component_oversized("src/App.test.tsx", &body).is_none());
}

#[test]
fn should_skip_non_frontend_files_for_component_oversized() {
    let mut body = "export function A() {}\nexport function B() {}\n".to_owned();
    body.push_str(&"// padding\n".repeat(100));
    assert!(check_component_oversized("src/main.rs", &body).is_none());
    assert!(check_component_oversized("src/config.json", &body).is_none());
}
