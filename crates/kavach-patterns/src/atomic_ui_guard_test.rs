use crate::atomic_ui_guard::detect;

#[test]
fn detects_atom_importing_molecule_react() {
    let v = detect(
        "src/ui/atoms/Button.tsx",
        "import { SearchBar } from '../molecules/SearchBar';",
    );
    assert!(v.iter().any(|x| x.pattern == "atom imports molecule"));
}

#[test]
fn detects_atom_importing_organism_vue() {
    let v = detect(
        "src/atoms/Logo.vue",
        "<script>import H from '@/organisms/Header.vue';</script>",
    );
    assert!(v.iter().any(|x| x.pattern == "atom imports organism"));
}

#[test]
fn detects_atom_importing_organism_svelte() {
    let v = detect(
        "src/atoms/Cell.svelte",
        "<script>import D from '../organisms/DataTable.svelte';</script>",
    );
    assert!(v.iter().any(|x| x.pattern == "atom imports organism"));
}

#[test]
fn detects_atom_importing_organism_dioxus() {
    let v = detect(
        "src/ui/atoms/button.rs",
        "use crate::organisms::Header; pub fn Button() -> Element { rsx! {} }",
    );
    assert!(v.iter().any(|x| x.pattern == "atom imports organism"));
}

#[test]
fn detects_atom_using_state_store() {
    let v = detect(
        "src/atoms/Button.tsx",
        "import { useStore } from '@/store'; export const Button = () => { useStore(); return <button />; };",
    );
    assert!(v.iter().any(|x| x.pattern == "atom uses state store"));
}

#[test]
fn detects_atom_calling_api() {
    let v = detect(
        "src/atoms/Avatar.tsx",
        "export const Avatar = async () => { await fetch('/api/user'); return <img alt='' />; };",
    );
    assert!(v.iter().any(|x| x.pattern == "atom calls API"));
}

#[test]
fn detects_molecule_importing_organism() {
    let v = detect(
        "src/molecules/SearchBar.tsx",
        "import H from '../organisms/Header'; export const S = () => <H />;",
    );
    assert!(v.iter().any(|x| x.pattern == "molecule imports organism"));
}

#[test]
fn allows_molecule_importing_atom() {
    let v = detect(
        "src/molecules/Form.tsx",
        "import { Button } from '../atoms/Button'; export const F = () => <Button alt='' />;",
    );
    assert!(!v.iter().any(|x| x.pattern.contains("imports")));
}

#[test]
fn detects_organism_importing_template() {
    let v = detect(
        "src/organisms/Header.tsx",
        "import { M } from '../templates/MainTemplate'; export const H = () => <M />;",
    );
    assert!(
        v.iter()
            .any(|x| x.pattern == "organism imports template/page")
    );
}

#[test]
fn detects_img_without_alt() {
    let v = detect(
        "src/components/Avatar.tsx",
        "export const A = () => <img src='/a.png' />;",
    );
    assert!(v.iter().any(|x| x.pattern == "img without alt"));
}

#[test]
fn allows_img_with_alt() {
    let v = detect(
        "src/components/Avatar.tsx",
        "export const A = () => <img src='/a.png' alt='User' />;",
    );
    assert!(!v.iter().any(|x| x.pattern == "img without alt"));
}

#[test]
fn detects_icon_button_without_aria_label() {
    let v = detect(
        "src/components/Close.tsx",
        "export const C = () => <button onClick={x}><svg /></button>;",
    );
    assert!(
        v.iter()
            .any(|x| x.pattern == "icon button without aria-label")
    );
}

#[test]
fn allows_icon_button_with_aria_label() {
    let v = detect(
        "src/components/Close.tsx",
        "export const C = () => <button aria-label='Close' onClick={x}><svg /></button>;",
    );
    assert!(
        !v.iter()
            .any(|x| x.pattern == "icon button without aria-label")
    );
}

#[test]
fn detects_list_without_key() {
    let v = detect(
        "src/components/List.tsx",
        "export const L = ({items}) => items.map(x => <li>{x}</li>);",
    );
    assert!(v.iter().any(|x| x.pattern == "list without key"));
}

#[test]
fn allows_list_with_key() {
    let v = detect(
        "src/components/List.tsx",
        "export const L = ({items}) => items.map(x => <li key={x.id}>{x.name}</li>);",
    );
    assert!(!v.iter().any(|x| x.pattern == "list without key"));
}

#[test]
fn detects_inline_style_with_hex() {
    let v = detect(
        "src/components/Card.tsx",
        "export const C = () => <div alt='' style={{color:'#ff0000'}}>x</div>;",
    );
    assert!(
        v.iter()
            .any(|x| x.pattern == "inline style with hardcoded color")
    );
}

#[test]
fn detects_arbitrary_px() {
    let v = detect(
        "src/components/Box.tsx",
        "export const B = () => <div alt='' className='p-[13px]'>x</div>;",
    );
    assert!(v.iter().any(|x| x.pattern == "arbitrary px value"));
}

#[test]
fn detects_missing_dark_mode() {
    let v = detect(
        "src/components/Card.tsx",
        "export const C = () => <div alt='' className='bg-white text-black'>x</div>;",
    );
    assert!(v.iter().any(|x| x.pattern == "missing dark mode pairing"));
}

#[test]
fn allows_paired_dark_mode() {
    let v = detect(
        "src/components/Card.tsx",
        "export const C = () => <div alt='' className='bg-white dark:bg-gray-900'>x</div>;",
    );
    assert!(!v.iter().any(|x| x.pattern == "missing dark mode pairing"));
}

#[test]
fn detects_atom_using_localstorage() {
    let v = detect(
        "src/atoms/Theme.tsx",
        "export const T = () => { const t = localStorage.getItem('theme'); return <span>{t}</span>; };",
    );
    assert!(v.iter().any(|x| x.pattern == "atom uses storage"));
}

#[test]
fn detects_console_log() {
    let v = detect(
        "src/components/Card.tsx",
        "export const C = () => { console.log('x'); return <div alt='' />; };",
    );
    assert!(v.iter().any(|x| x.pattern == "debug logging in component"));
}

#[test]
fn non_ui_file_skipped() {
    let v = detect(
        "src/utils/math.ts",
        "export const add = (a:number,b:number) => a+b;",
    );
    assert!(v.is_empty());
}

#[test]
fn test_file_skipped() {
    let v = detect(
        "/project/tests/Button.test.tsx",
        "import x from '../organisms/y';",
    );
    assert!(v.is_empty());
}
