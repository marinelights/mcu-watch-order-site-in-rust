//! Road to Doomsday: the MCU watch planner, written in Rust and compiled to WebAssembly.
//!
//! The page's HTML shell and CSS live in `shell.html`; everything that runs
//! (state, saving, the five looks, the title picker, prerequisites and the map)
//! is in this file. `build.mjs` compiles it and packs it into one `index.html`.

use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    Document, Element, Event, HtmlElement, KeyboardEvent, ScrollBehavior,
    ScrollIntoViewOptions, ScrollLogicalPosition, ScrollToOptions, Window,
};

// =====================================================================
// Data
// =====================================================================

const RAW: &str = include_str!("data.txt");
const STORE_KEY: &str = "mcu-wl2";

pub struct Title {
    pub i: usize,
    pub k: &'static str,
    pub t: char,
    pub n: &'static str,
    pub d: &'static str,
    pub m: u32,
    pub e: bool,
    pub must: Vec<usize>,
    pub sh: Vec<usize>,
}

fn data() -> &'static [Title] {
    static D: OnceLock<Vec<Title>> = OnceLock::new();
    D.get_or_init(parse)
}

fn parse() -> Vec<Title> {
    let lines: Vec<&'static str> = RAW.lines().filter(|l| !l.trim().is_empty()).collect();
    let keys: HashMap<&str, usize> = lines
        .iter()
        .enumerate()
        .map(|(i, l)| (l.split('|').next().unwrap_or(""), i))
        .collect();
    let sp = |s: &str| -> Vec<usize> {
        if s.is_empty() {
            vec![]
        } else {
            s.split(',').filter_map(|k| keys.get(k).copied()).collect()
        }
    };
    lines
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let a: Vec<&'static str> = l.split('|').collect();
            let f = |n: usize| a.get(n).copied().unwrap_or("");
            Title {
                i,
                k: f(0),
                t: f(1).chars().next().unwrap_or('F'),
                n: f(2),
                d: f(3),
                m: f(4).parse().unwrap_or(0),
                e: !f(5).is_empty(),
                must: sp(f(6)),
                sh: sp(f(7)),
            }
        })
        .collect()
}

fn idx(k: &str) -> usize {
    data().iter().position(|x| x.k == k).unwrap_or(0)
}

/// Direct prerequisites: (title, weight) where 2 = must, 1 = should.
fn edges(i: usize) -> Vec<(usize, u8)> {
    let x = &data()[i];
    x.must.iter().map(|&a| (a, 2)).chain(x.sh.iter().map(|&a| (a, 1))).collect()
}

/// Every title that leads (directly or indirectly) into `i`.
fn reach(i: usize) -> &'static HashSet<usize> {
    static R: OnceLock<Vec<HashSet<usize>>> = OnceLock::new();
    &R.get_or_init(|| {
        fn go(i: usize, memo: &mut Vec<Option<HashSet<usize>>>) -> HashSet<usize> {
            if let Some(s) = &memo[i] {
                return s.clone();
            }
            let mut s = HashSet::new();
            for (a, _) in edges(i) {
                s.insert(a);
                s.extend(go(a, memo));
            }
            memo[i] = Some(s.clone());
            s
        }
        let n = data().len();
        let mut memo = vec![None; n];
        (0..n).map(|i| go(i, &mut memo)).collect()
    })[i]
}

/// All ancestors of `t`, each with the strongest link strength along any path.
fn anc(t: usize) -> HashMap<usize, u8> {
    fn go(k: usize, v: u8, s: &mut HashMap<usize, u8>) {
        for (a, w) in edges(k) {
            let nv = v.min(w);
            if *s.get(&a).unwrap_or(&0) < nv {
                s.insert(a, nv);
                go(a, nv, s);
            }
        }
    }
    let mut s = HashMap::new();
    go(t, 2, &mut s);
    s
}

// =====================================================================
// Text helpers
// =====================================================================

fn strand(t: char) -> &'static str {
    match t {
        'D' => "Disney+ series",
        'N' => "Netflix series",
        _ => "Film",
    }
}
fn shorts(t: char) -> &'static str {
    match t {
        'D' => "Disney+",
        'N' => "Netflix",
        _ => "Film",
    }
}
fn badge(t: char) -> Option<&'static str> {
    match t {
        'D' => Some("D+"),
        'N' => Some("NF"),
        _ => None,
    }
}
fn fmt(m: u32) -> String {
    format!("{}h {:02}m", m / 60, m % 60)
}
fn rt(x: &Title) -> String {
    if x.m > 0 {
        format!("{}{}", if x.e { "~" } else { "" }, fmt(x.m))
    } else {
        "In cinemas".into()
    }
}
fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('"', "&quot;")
}
fn short(n: &str) -> &str {
    match n.find(" (") {
        Some(p) => &n[..p],
        None => n,
    }
}
fn paren(n: &str) -> Option<&str> {
    let a = n.find('(')?;
    let b = n.rfind(')')?;
    (b > a).then(|| &n[a + 1..b])
}
fn pad(n: usize) -> String {
    format!("{:02}", n)
}
fn year(x: &Title) -> &'static str {
    &x.d[x.d.len() - 4..]
}
fn initials(n: &str) -> String {
    short(n)
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| matches!(c, ':' | '*' | '“' | '”' | '"')))
        .filter(|w| !w.is_empty() && !matches!(w.to_lowercase().as_str(), "the" | "of" | "and" | "a"))
        .map(|w| {
            if w.chars().next().map_or(false, |c| c.is_ascii_digit()) {
                w.to_string()
            } else {
                w.chars().next().unwrap().to_string()
            }
        })
        .collect::<String>()
        .chars()
        .take(3)
        .collect::<String>()
        .to_uppercase()
}
const CK: &str = r##"<svg><use href="#i-check"/></svg>"##;
fn ico(id: &str) -> String {
    format!(r##"<svg class="i"><use href="#i-{id}"/></svg>"##)
}

// =====================================================================
// DOM helpers
// =====================================================================

fn win() -> Window {
    web_sys::window().expect("window")
}
fn doc() -> Document {
    win().document().expect("document")
}
fn el(id: &str) -> Element {
    doc().get_element_by_id(id).unwrap_or_else(|| panic!("#{id} missing"))
}
fn set_html(id: &str, h: &str) {
    el(id).set_inner_html(h);
}
fn qa_in(root: &Element, sel: &str) -> Vec<Element> {
    match root.query_selector_all(sel) {
        Ok(nl) => (0..nl.length())
            .filter_map(|i| nl.item(i))
            .filter_map(|n| n.dyn_into::<Element>().ok())
            .collect(),
        Err(_) => vec![],
    }
}
fn qa(sel: &str) -> Vec<Element> {
    match doc().query_selector_all(sel) {
        Ok(nl) => (0..nl.length())
            .filter_map(|i| nl.item(i))
            .filter_map(|n| n.dyn_into::<Element>().ok())
            .collect(),
        Err(_) => vec![],
    }
}
fn cls(e: &Element, c: &str, on: bool) {
    let _ = e.class_list().toggle_with_force(c, on);
}
fn has_cls(id: &str, c: &str) -> bool {
    el(id).class_list().contains(c)
}
fn attr(e: &Element, k: &str, v: &str) {
    let _ = e.set_attribute(k, v);
}
fn num_attr(e: &Element, k: &str) -> Option<usize> {
    e.get_attribute(k)?.parse().ok()
}
fn is_hidden(id: &str) -> bool {
    el(id).has_attribute("hidden")
}
fn set_hidden(id: &str, h: bool) {
    let e = el(id);
    if h {
        attr(&e, "hidden", "");
    } else {
        let _ = e.remove_attribute("hidden");
    }
}
fn on<F: FnMut(Event) + 'static>(target: &web_sys::EventTarget, ev: &str, f: F) {
    let c = Closure::<dyn FnMut(Event)>::new(f);
    let _ = target.add_event_listener_with_callback(ev, c.as_ref().unchecked_ref());
    c.forget();
}
fn on_id<F: FnMut(Event) + 'static>(id: &str, ev: &str, f: F) {
    on(&el(id), ev, f);
}
fn closest(e: &Event, sel: &str) -> Option<Element> {
    e.target()?.dyn_into::<Element>().ok()?.closest(sel).ok()?
}
fn timeout<F: FnOnce() + 'static>(ms: i32, f: F) {
    let c = Closure::once_into_js(f);
    let _ = win().set_timeout_with_callback_and_timeout_and_arguments_0(c.unchecked_ref(), ms);
}
fn raf<F: FnOnce() + 'static>(f: F) {
    let c = Closure::once_into_js(f);
    let _ = win().request_animation_frame(c.unchecked_ref());
}
fn media(q: &str) -> bool {
    win().match_media(q).ok().flatten().map(|m| m.matches()).unwrap_or(false)
}
fn reduced() -> bool {
    media("(prefers-reduced-motion: reduce)")
}
fn behavior() -> ScrollBehavior {
    if reduced() {
        ScrollBehavior::Auto
    } else {
        ScrollBehavior::Smooth
    }
}
fn scroll_y() -> f64 {
    win().scroll_y().unwrap_or(0.0)
}
fn scroll_to_y(y: f64) {
    let o = ScrollToOptions::new();
    o.set_top(y);
    o.set_behavior(behavior());
    win().scroll_to_with_scroll_to_options(&o);
}
fn into_view(e: &Element, block: ScrollLogicalPosition, smooth: bool) {
    let o = ScrollIntoViewOptions::new();
    o.set_block(block);
    o.set_behavior(if smooth { behavior() } else { ScrollBehavior::Auto });
    e.scroll_into_view_with_scroll_into_view_options(&o);
}
fn top_of(id: &str) -> f64 {
    el(id).get_bounding_client_rect().top() + scroll_y()
}
fn focus(e: &Element) {
    if let Some(h) = e.dyn_ref::<HtmlElement>() {
        let _ = h.focus();
    }
}
fn key_of(e: &Event) -> String {
    e.dyn_ref::<KeyboardEvent>().map(|k| k.key()).unwrap_or_default()
}
fn days_left() -> i64 {
    let now = js_sys::Date::now();
    let doom = js_sys::Date::new_with_year_month_day(2026, 11, 18).get_time();
    (((doom - now) / 864e5).ceil() as i64).max(0)
}

// =====================================================================
// Saved state
// =====================================================================

#[derive(Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "camelCase")]
struct Saved {
    done: Option<Vec<usize>>,
    skip_n: bool,
    tg: serde_json::Value,
    hide_s: bool,
    view: String,
    pv: Option<String>,
    map_hide: bool,
    theme: String,
}
impl Default for Saved {
    fn default() -> Self {
        Saved {
            done: None,
            skip_n: false,
            tg: serde_json::Value::from("BD"),
            hide_s: false,
            view: "L".into(),
            pv: None,
            map_hide: true,
            theme: "programme".into(),
        }
    }
}

struct Graph {
    p: HashMap<usize, Vec<(usize, u8)>>,
    edges: Vec<(usize, usize, u8)>,
}

struct App {
    st: Saved,
    tg: usize,
    last_next: Option<usize>,
    booted: bool,
    /// Whether a title is selected in the prerequisites chart.
    sel_on: bool,
    /// The laid-out chart; None means it must be rebuilt before showing.
    graph: Option<Graph>,
}

thread_local! {
    static APP: RefCell<App> = RefCell::new(App::load());
}

/// Run `f` against the app. Returns None if the app is already busy
/// (an event fired while another handler was running), so nothing re-enters.
fn with<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|a| a.try_borrow_mut().ok().map(|mut g| f(&mut g)))
}

struct Ctx {
    v: Vec<usize>,
    nx: Option<usize>,
    left: u32,
    cnt: usize,
    days: i64,
    per_day: u32,
    pos: usize,
    swap: &'static str,
}

struct Group {
    yr: &'static str,
    n: usize,
    items: Vec<(usize, usize)>,
}

const LOOKS: [(&str, &str, &str, [&str; 3]); 5] = [
    ("programme", "Programme", "Cinema schedule: ticket stub, year column, dotted leaders", ["#3b0c13", "#f3e7d2", "#ffc61a"]),
    ("poster", "Poster", "Swiss grid: runtime barcode, numbered tiles, colour bands", ["#e8eaed", "#0c0c0e", "#2443ff"]),
    ("comic", "Comic", "Starburst title, power meter, panels with SEEN! stamps", ["#ffd43b", "#111111", "#e4222d"]),
    ("night", "Night", "Dark app: sidebar with progress ring, table with switches", ["#0c1120", "#ff7a66", "#3fd0c9"]),
    ("studio", "Studio", "Light app: swipeable up-next cards, timeline, floating dock", ["#f4f4f2", "#121214", "#ff3b30"]),
];
fn sw_html(sw: &[&str; 3]) -> String {
    sw.iter().map(|c| format!(r##"<i style="background:{c}"></i>"##)).collect()
}

fn burst_pts() -> &'static str {
    static B: OnceLock<String> = OnceLock::new();
    B.get_or_init(|| {
        let (n, w, h) = (22usize, 600.0f64, 400.0f64);
        (0..n * 2)
            .map(|i| {
                let a = i as f64 / (n * 2) as f64 * std::f64::consts::TAU;
                let r = if i % 2 == 1 { 0.74 } else { 0.94 + ((i * 37) % 7) as f64 / 100.0 };
                format!("{:.1},{:.1}", w / 2.0 + a.cos() * w / 2.0 * r, h / 2.0 + a.sin() * h / 2.0 * r)
            })
            .collect::<Vec<_>>()
            .join(" ")
    })
}

impl App {
    fn load() -> App {
        let mut st: Saved = win()
            .local_storage()
            .ok()
            .flatten()
            .and_then(|s| s.get_item(STORE_KEY).ok().flatten())
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_default();
        if st.done.is_none() {
            st.done = Some((0..6).collect());
        }
        // `tg` was a title key in older saves and an index now; accept either.
        let tg = match &st.tg {
            serde_json::Value::String(k) => idx(k),
            serde_json::Value::Number(n) => n.as_u64().map(|n| n as usize).filter(|&n| n < data().len()).unwrap_or_else(|| idx("BD")),
            _ => idx("BD"),
        };
        App { st, tg, last_next: None, booted: false, sel_on: true, graph: None }
    }
    fn save(&mut self) {
        self.st.tg = serde_json::Value::from(data()[self.tg].k);
        if let (Ok(j), Some(s)) = (serde_json::to_string(&self.st), win().local_storage().ok().flatten()) {
            let _ = s.set_item(STORE_KEY, &j);
        }
    }
    fn done(&self) -> &Vec<usize> {
        self.st.done.as_ref().expect("done list")
    }
    fn dn(&self, i: usize) -> bool {
        self.done().contains(&i)
    }
    fn toggle(&mut self, i: usize) {
        let d = self.st.done.get_or_insert_with(Vec::new);
        if let Some(p) = d.iter().position(|&x| x == i) {
            d.remove(p);
        } else {
            d.push(i);
        }
        self.save();
        // watched ticks show in the chart, so lay it out again next time it's shown
        self.graph = None;
    }
    fn vis(&self) -> Vec<usize> {
        data()
            .iter()
            .filter(|x| x.t != 'U' && !(self.st.skip_n && x.t == 'N'))
            .map(|x| x.i)
            .collect()
    }
    fn next_up(&self) -> Option<usize> {
        self.vis().into_iter().find(|&i| !self.dn(i))
    }

    // ---------------- looks ----------------
    fn apply_look(&mut self, id: &str) {
        let look = LOOKS.iter().find(|l| l.0 == id).unwrap_or(&LOOKS[0]);
        self.st.theme = look.0.into();
        if let Some(root) = doc().document_element() {
            attr(&root, "data-theme", look.0);
        }
        el("lookName").set_text_content(Some(look.1));
        set_html("lookSw", &sw_html(&look.3));
        attr(&el("themeColor"), "content", look.3[0]);
        let opts: String = LOOKS
            .iter()
            .map(|l| {
                format!(
                    r##"<button class="look-opt" role="menuitemradio" aria-checked="{}" data-id="{}"><span class="sws">{}</span><b>{}</b><span class="d">{}</span><svg class="i ck"><use href="#i-check"/></svg></button>"##,
                    l.0 == look.0, l.0, sw_html(&l.3), l.1, l.2
                )
            })
            .collect();
        set_html("lookPop", &opts);
        if self.booted {
            self.last_next = None;
            self.render_top();
            if self.st.view == "L" {
                self.render_list();
            } else {
                self.render_flow();
            }
        }
        raf(|| {
            thumbs();
            with(|a| a.draw());
        });
    }

    // ---------------- hero ----------------
    fn ctx(&self) -> Ctx {
        let v = self.vis();
        let nx = self.next_up();
        let (mut left, mut cnt) = (0u32, 0usize);
        for &i in &v {
            if self.dn(i) {
                cnt += 1;
            } else {
                left += data()[i].m;
            }
        }
        let days = days_left();
        let per_day = if days > 0 { (left as f64 / days as f64).ceil() as u32 } else { left };
        let pos = nx.and_then(|n| v.iter().position(|&i| i == n)).map_or(0, |p| p + 1);
        let swap = match (self.last_next, nx) {
            (Some(a), Some(b)) if a != b && !reduced() => "swap",
            _ => "",
        };
        Ctx { v, nx, left, cnt, days, per_day, pos, swap }
    }
    fn notice(&self, c: &Ctx) -> String {
        if c.left > 0 {
            let tail = if c.days > 0 {
                format!(
                    "Doomsday opens 18 December, in {} day{}, <em>so that's about {} a night.</em>",
                    c.days,
                    if c.days > 1 { "s" } else { "" },
                    fmt(c.per_day)
                )
            } else {
                "<em>Doomsday is out.</em>".into()
            };
            format!("<b>{}</b> left to watch. You've seen {} of {}. {}", fmt(c.left), c.cnt, c.v.len(), tail)
        } else {
            format!("<b>All {} watched.</b> <em>You're ready for Doomsday.</em>", c.v.len())
        }
    }
    fn cell_cls(&self, i: usize, c: &Ctx) -> &'static str {
        if self.dn(i) {
            "s"
        } else if Some(i) == c.nx {
            "x"
        } else {
            ""
        }
    }
    fn hero(&self, c: &Ctx) -> String {
        let d = data();
        let title = esc(&nx_title(c));
        let meta = nx_meta(c);
        match self.st.theme.as_str() {
            "poster" => {
                let mx = c.v.iter().map(|&i| d[i].m).max().unwrap_or(1).max(1) as f64;
                let bars: String = c.v.iter().map(|&i| {
                    let x = &d[i];
                    format!(r##"<i class="{}" data-jump="{}" style="height:{:.1}%" title="{} · {}"></i>"##,
                        self.cell_cls(i, c), i, 18.0 + 82.0 * (x.m as f64 / mx).sqrt(), esc(x.n), rt(x))
                }).collect();
                format!(
                    r##"<header class="p-hero"><h1 class="disp">Road to <span>Doomsday</span></h1>
<div class="p-stats"><div><b>{left}</b><span>Left to watch</span></div><div><b>{cnt}/{tot}</b><span>Watched</span></div><div><b>{days}</b><span>Days to Doomsday</span></div></div>
<div><div class="barcode">{bars}</div><div class="bc-key"><span>2008</span><span>Bar height = runtime</span><span>2026</span></div></div>
<section class="p-next" aria-label="Next up"><div class="p-num">{num}</div><div class="k">{k}</div><h2 class="disp {swap}">{title}</h2><div class="meta">{meta}</div>{acts}</section></header>"##,
                    left = fmt(c.left), cnt = c.cnt, tot = c.v.len(), days = c.days, bars = bars,
                    num = if c.nx.is_some() { pad(c.pos) } else { "✓".into() },
                    k = if c.nx.is_some() { "Next up" } else { "Complete" },
                    swap = c.swap, title = title, meta = meta, acts = acts(c, "btn-dark", "btn-line")
                )
            }
            "comic" => {
                let frac = c.cnt as f64 / c.v.len().max(1) as f64;
                let on = (frac * 20.0).round() as usize;
                let pct = (frac * 100.0).round();
                let segs: String = (0..20).map(|i| if i < on { r#"<i class="on"></i>"# } else { "<i></i>" }).collect();
                let cap = if c.left > 0 {
                    format!("you've got <b>{}</b> of Marvel left. {} of {} down, and Doomsday hits cinemas in <b>{} days</b>. That's about {} a night!",
                        fmt(c.left), c.cnt, c.v.len(), c.days, fmt(c.per_day))
                } else {
                    format!("you've watched <b>all {}</b>. Doomsday, here we come!", c.v.len())
                };
                format!(
                    r##"<header class="c-hero">
<div class="burst"><svg viewBox="0 0 600 400" preserveAspectRatio="none" aria-hidden="true"><polygon points="{pts}" transform="translate(12 12)" fill="#111"/><polygon points="{pts}" fill="#fff" stroke="#111" stroke-width="6" stroke-linejoin="round"/></svg><h1 class="disp">Road to Doomsday!<span>Every MCU film &amp; series, in order</span></h1></div>
<div class="c-side"><div class="caption"><span class="lbl">Meanwhile…</span>{cap}</div>
<div class="meter-w"><span class="lbl">Power level</span><div class="meter" role="img" aria-label="{pct}% watched">{segs}</div><span class="lbl">{pct}%</span></div></div>
<section class="c-next" aria-label="Next up"><div class="bubble lbl">{bub}</div><h2 class="disp {swap}">{title}</h2><div class="meta">{meta}</div>{acts}</section></header>"##,
                    pts = burst_pts(), cap = cap, pct = pct, segs = segs,
                    bub = if c.nx.is_some() { "Next up!" } else { "You did it!" },
                    swap = c.swap, title = title, meta = meta, acts = acts(c, "btn-dark", "btn-line")
                )
            }
            "night" => {
                let q: Vec<usize> = c.v.iter().copied().filter(|&i| !self.dn(i)).skip(1).take(3).collect();
                let queue = if q.is_empty() {
                    String::new()
                } else {
                    let rows: String = q.iter().map(|&i| {
                        let x = &d[i];
                        format!(r##"<div class="n-qi" data-act="jumpq" data-i="{}"><i class="dot {}"></i><b>{}</b><span>{}</span></div>"##, i, x.t, esc(short(x.n)), rt(x))
                    }).collect();
                    format!(r##"<div class="n-q"><h3>Coming after</h3>{rows}</div>"##)
                };
                let (mono_t, mono) = match c.nx { Some(n) => (d[n].t, initials(d[n].n)), None => ('F', "✓".into()) };
                format!(
                    r##"<header class="n-hero"><div><h1 class="disp">Road to Doomsday</h1><p class="dek">{dek}</p></div>
<section class="n-next card" aria-label="Next up"><div class="mono {mt}">{mono}</div><div><div class="up">{up}</div><h2 class="disp {swap}">{title}</h2><div class="meta">{meta}</div></div>{acts}</section>{queue}</header>"##,
                    dek = self.notice(c).replace("<em>", "").replace("</em>", ""),
                    mt = mono_t, mono = mono,
                    up = if c.nx.is_some() { format!("Up next · #{}", c.pos) } else { "All done".into() },
                    swap = c.swap, title = title, meta = meta, acts = acts(c, "btn-pri", "btn-sec"), queue = queue
                )
            }
            "studio" => {
                let up: Vec<usize> = c.v.iter().copied().filter(|&i| !self.dn(i)).take(6).collect();
                let cards: String = if up.is_empty() {
                    format!(r##"<article class="s-card first"><span class="no">All done</span><b>See you at Doomsday</b><span class="sub">18 December 2026</span>{}</article>"##, acts(c, "btn-dark", "btn-line"))
                } else {
                    up.iter().enumerate().map(|(n, &i)| {
                        let x = &d[i];
                        if n == 0 {
                            format!(r##"<article class="s-card first"><span class="no">Up next · No. {}</span><b class="{}">{}</b><span class="sub">{} · {} · {}</span>{}</article>"##,
                                c.pos, c.swap, esc(short(x.n)), x.d, strand(x.t), rt(x), acts(c, "btn-dark", "btn-line"))
                        } else {
                            let p = c.v.iter().position(|&j| j == i).map_or(0, |p| p + 1);
                            format!(r##"<article class="s-card"><span class="no">No. {}</span><b>{}</b><span class="sub">{} · {} · {}</span><button class="s-tog" data-act="done" data-i="{}">{}Mark watched</button></article>"##,
                                p, esc(short(x.n)), x.d, shorts(x.t), rt(x), i, ico("check"))
                        }
                    }).collect()
                };
                let pct = c.cnt as f64 / c.v.len().max(1) as f64 * 100.0;
                format!(
                    r##"<header class="s-hero"><h1 class="disp">Road to <span>Doomsday</span></h1>
<div class="s-prog"><div class="s-bar"><i style="width:{pct:.1}%"></i></div><div class="s-chips"><span>{cnt} of {tot} watched</span><span><b>{left}</b> left</span><span>{days} days to Doomsday</span><span>≈ {pd} a night</span></div></div>
<div class="s-car" aria-label="Up next">{cards}</div></header>"##,
                    pct = pct, cnt = c.cnt, tot = c.v.len(), left = fmt(c.left), days = c.days, pd = fmt(c.per_day), cards = cards
                )
            }
            _ => {
                let reel: String = c.v.iter().map(|&i| {
                    format!(r##"<i class="{}" data-jump="{}" title="{} ({})"></i>"##, self.cell_cls(i, c), i, esc(d[i].n), d[i].d)
                }).collect();
                format!(
                    r##"<header class="mast"><h1 class="disp">Road to <span>Doomsday</span></h1><div class="side"><p class="dek">Every Marvel film and series in release order, 2008 to 2026, and what to watch before any of them.</p><p class="notice">{notice}</p></div></header>
<div class="prog card"><div class="reel" style="--n:{n};--half:{half}">{reel}</div><div class="reel-key"><span>2008</span><span>2026</span></div></div>
<section class="slab" aria-label="Next up"><div class="slab-no lbl">{no}</div><h2 class="disp {swap}">{title}</h2><div class="meta">{meta}</div>{acts}</section>"##,
                    notice = self.notice(c), n = c.v.len(), half = (c.v.len() + 1) / 2, reel = reel,
                    no = if c.nx.is_some() { format!("Next up · No. {} of {}", c.pos, c.v.len()) } else { "Season complete".into() },
                    swap = c.swap, title = title, meta = meta, acts = acts(c, "btn-dark", "btn-line")
                )
            }
        }
    }
    fn side_html(&self, c: &Ctx) -> String {
        let p = c.cnt as f64 / c.v.len().max(1) as f64;
        let circ = 2.0 * std::f64::consts::PI * 58.0;
        format!(
            r##"<div class="ring"><svg viewBox="0 0 140 140"><circle class="tr" cx="70" cy="70" r="58" fill="none" stroke-width="12"/><circle class="vl" cx="70" cy="70" r="58" fill="none" stroke-width="12" stroke-dasharray="{c:.1}" stroke-dashoffset="{o:.1}"/></svg><div><b>{pct}%</b><span>watched</span></div></div>
<div class="side-stats"><div>Left to watch<b>{left}</b></div><div>Titles seen<b>{cnt} / {tot}</b></div><div>Doomsday in<b>{days} days</b></div><div>Per night<b>{pd}</b></div></div>"##,
            c = circ, o = circ * (1.0 - p), pct = (p * 100.0).round(), left = fmt(c.left), cnt = c.cnt, tot = c.v.len(), days = c.days, pd = fmt(c.per_day)
        )
    }
    fn render_top(&mut self) {
        let c = self.ctx();
        set_html("hero", &self.hero(&c));
        set_html("side", &self.side_html(&c));
        set_html("barLeft", &format!("<b>{}</b>left", fmt(c.left)));
        self.last_next = c.nx;
    }
    fn refresh(&mut self) {
        self.render_top();
        if self.st.view == "L" {
            if self.st.hide_s {
                self.render_list();
            } else {
                self.mark_next();
            }
        } else {
            self.render_flow();
        }
    }

    // ---------------- watchlist ----------------
    fn punch(&self, i: usize, extra: &str) -> String {
        let d = self.dn(i);
        format!(
            r##"<button class="punch {extra}" aria-pressed="{d}" aria-label="{lab} {n} watched">{inner}</button>"##,
            extra = extra, d = d, lab = if d { "Unmark" } else { "Mark" }, n = esc(data()[i].n),
            inner = if extra.contains("sw") { "<i></i>" } else { CK }
        )
    }
    fn row_html(&self, i: usize, pos: usize, tag: Option<String>) -> String {
        let x = &data()[i];
        let tag = tag.unwrap_or_else(|| format!(r##"<span class="tag lbl {}">{}</span>"##, x.t, strand(x.t)));
        format!(
            r##"<li class="sc{seen}" data-i="{i}">{punch}<div><div class="t-line"><span class="t-title"><span class="tt">{n}</span></span><span class="lead"></span><span class="rt">{rt}</span></div><div class="t-meta">{tag}<span>{d}</span>{pos}</div></div>{pre}</li>"##,
            seen = if self.dn(i) { " seen" } else { "" }, i = i, punch = self.punch(i, ""), n = esc(x.n), rt = rt(x),
            tag = tag, d = x.d, pos = if pos > 0 { format!("<span>No. {pos}</span>") } else { String::new() }, pre = pre_btn(i, "Prereqs")
        )
    }
    fn list_html(&self, groups: &[Group]) -> String {
        let d = data();
        let seen = |i: usize| if self.dn(i) { " seen" } else { "" };
        let mins = |g: &Group| g.items.iter().map(|&(i, _)| d[i].m).sum::<u32>();
        match self.st.theme.as_str() {
            "poster" => groups.iter().map(|g| {
                let tiles: String = g.items.iter().map(|&(i, pos)| {
                    let x = &d[i];
                    format!(r##"<div class="tile{s}" data-i="{i}"><div class="tile-top"><span class="tile-no">{no}</span>{p}</div><div class="tile-name">{n}</div><div class="tile-sub"><span class="tag lbl {t}">{st}</span>{d}</div><div class="tile-foot"><span class="rt">{rt}</span>{pre}</div></div>"##,
                        s = seen(i), i = i, no = pad(pos), p = self.punch(i, ""), n = esc(x.n), t = x.t, st = shorts(x.t), d = x.d, rt = rt(x), pre = pre_btn(i, ""))
                }).collect();
                format!(r##"<section class="p-year" style="--yc:var(--yc{c})"><h3 class="disp">{yr}<small>{n} title{pl} · {m}</small></h3><div class="p-grid">{tiles}</div></section>"##,
                    c = g.n % 6, yr = g.yr, n = g.items.len(), pl = if g.items.len() > 1 { "s" } else { "" }, m = fmt(mins(g)), tiles = tiles)
            }).collect(),
            "comic" => groups.iter().map(|g| {
                let pans: String = g.items.iter().map(|&(i, pos)| {
                    let x = &d[i];
                    format!(r##"<div class="pan{w}{s}" data-i="{i}"><span class="pan-cap lbl">#{pos} · {d}</span><div class="pan-t disp">{n}</div><div class="pan-m">{st} · {rt}</div>{p}{pre}<span class="stamp lbl" aria-hidden="true">Seen!</span><span class="here lbl" aria-hidden="true">Next!</span></div>"##,
                        w = if x.m >= 300 { " wide" } else { "" }, s = seen(i), i = i, pos = pos, d = x.d, n = esc(x.n), st = strand(x.t), rt = rt(x), p = self.punch(i, ""), pre = pre_btn(i, ""))
                }).collect();
                format!(r##"<section class="c-page"><h3 class="c-cap disp">{}</h3><div class="c-grid">{}</div></section>"##, g.yr, pans)
            }).collect(),
            "night" => groups.iter().map(|g| {
                let rows: String = g.items.iter().map(|&(i, _)| {
                    let x = &d[i];
                    format!(r##"<div class="n-row{s}" data-i="{i}">{p}<div class="n-t"><span class="n-name">{n}</span><span class="pill {t}">{st}</span></div><span class="n-d">{d}</span><span class="n-r">{rt}</span>{pre}</div>"##,
                        s = seen(i), i = i, p = self.punch(i, "sw"), n = esc(x.n), t = x.t, st = shorts(x.t), d = x.d, rt = rt(x), pre = pre_btn(i, "Prereqs"))
                }).collect();
                let w = g.items.iter().filter(|&&(i, _)| self.dn(i)).count();
                format!(r##"<section class="n-grp" style="--yc:var(--yc{c})"><h3>{yr}<small>{w}/{n} watched · {m}</small></h3><div class="n-table">{rows}</div></section>"##,
                    c = g.n % 6, yr = g.yr, w = w, n = g.items.len(), m = fmt(mins(g)), rows = rows)
            }).collect(),
            "studio" => {
                let mut k = 0usize;
                let mut h = String::from(r#"<ol class="tl">"#);
                for g in groups {
                    h += &format!(r##"<li class="tl-yr"><span>{}</span></li>"##, g.yr);
                    for &(i, pos) in &g.items {
                        let x = &d[i];
                        h += &format!(r##"<li class="tl-it {side}{s}" data-i="{i}">{p}<div class="tl-card"><div class="tl-head"><span>No. {pos}</span><span class="tag lbl {t}">{st}</span></div><div class="tl-name">{n}</div><div class="tl-sub">{d} · {rt}</div>{pre}</div></li>"##,
                            side = if k % 2 == 1 { "r" } else { "l" }, s = seen(i), i = i, p = self.punch(i, ""), pos = pos, t = x.t, st = shorts(x.t), n = esc(x.n), d = x.d, rt = rt(x), pre = pre_btn(i, "Prereqs"));
                        k += 1;
                    }
                }
                h + "</ol>"
            }
            _ => groups.iter().map(|g| {
                let rows: String = g.items.iter().map(|&(i, pos)| self.row_html(i, pos, None)).collect();
                format!(r##"<div class="yr-block"><h3 class="yr disp">{}</h3><ol class="screens">{}</ol></div>"##, g.yr, rows)
            }).collect(),
        }
    }
    fn render_list(&mut self) {
        let mut groups: Vec<Group> = vec![];
        for (n, i) in self.vis().into_iter().enumerate() {
            if self.st.hide_s && self.dn(i) {
                continue;
            }
            let yr = year(&data()[i]);
            if groups.last().map_or(true, |g| g.yr != yr) {
                let gn = groups.len();
                groups.push(Group { yr, n: gn, items: vec![] });
            }
            groups.last_mut().unwrap().items.push((i, n + 1));
        }
        let h = if groups.is_empty() {
            r#"<div class="empty disp">Nothing left on the programme.<br>See you at Doomsday.</div>"#.to_string()
        } else {
            self.list_html(&groups)
        };
        set_html("list", &h);
        self.mark_next();
    }
    fn mark_next(&self) {
        let nx = self.next_up();
        for r in qa("#list [data-i]") {
            let Some(i) = num_attr(&r, "data-i") else { continue };
            let d = self.dn(i);
            cls(&r, "seen", d);
            cls(&r, "next", Some(i) == nx);
            if let Ok(Some(p)) = r.query_selector(".punch") {
                attr(&p, "aria-pressed", if d { "true" } else { "false" });
                attr(&p, "aria-label", &format!("{} {} watched", if d { "Unmark" } else { "Mark" }, data()[i].n));
            }
        }
        if self.st.theme == "night" {
            for g in qa(".n-grp") {
                let rows = qa_in(&g, ".n-row");
                let ids: Vec<usize> = rows.iter().filter_map(|r| num_attr(r, "data-i")).collect();
                let w = ids.iter().filter(|&&i| self.dn(i)).count();
                let m: u32 = ids.iter().map(|&i| data()[i].m).sum();
                if let Ok(Some(s)) = g.query_selector("h3 small") {
                    s.set_text_content(Some(&format!("{}/{} watched · {}", w, ids.len(), fmt(m))));
                }
            }
        }
    }
    fn scroll_to_row(&mut self, i: usize) {
        if self.st.view != "L" {
            self.show("L");
        }
        let sel = format!(r#"#list [data-i="{i}"]"#);
        let find = || doc().query_selector(&sel).ok().flatten();
        let mut r = find();
        if r.is_none() && self.st.hide_s {
            self.st.hide_s = false;
            self.sync_chips();
            self.save();
            self.render_list();
            r = find();
        }
        if r.is_none() && self.st.skip_n {
            self.st.skip_n = false;
            self.sync_chips();
            self.save();
            self.render_top();
            self.render_list();
            r = find();
        }
        if let Some(r) = r {
            into_view(&r, ScrollLogicalPosition::Center, true);
        }
    }
    fn sync_chips(&self) {
        attr(&el("incN"), "aria-pressed", if self.st.skip_n { "false" } else { "true" });
        attr(&el("hideS"), "aria-pressed", if self.st.hide_s { "true" } else { "false" });
        attr(&el("mapHide"), "aria-pressed", if self.st.map_hide { "true" } else { "false" });
    }
    /// Open the prerequisites tab with `k` selected, scrolled so its chart card is in view.
    fn plan_for(&mut self, k: usize) {
        self.tg = k;
        self.sel_on = true;
        self.save();
        self.show("F");
        timeout(0, move || {
            let node = doc().get_element_by_id(&format!("n_{k}")).filter(|_| !has_cls("mapWrap", "hide"));
            match node {
                Some(n) => into_view(&n, ScrollLogicalPosition::Center, true),
                None => scroll_to_y(top_of("vF") - 80.0),
            }
        });
    }
    fn select(&mut self, k: Option<usize>) {
        match k {
            Some(k) => {
                self.sel_on = !(self.sel_on && self.tg == k); // tapping the selected title again clears it
                self.tg = k;
            }
            None => self.sel_on = false,
        }
        self.save();
        self.render_flow();
    }

    // ---------------- prerequisites ----------------
    fn pv_mode(&self) -> &str {
        match self.st.pv.as_deref() {
            Some("L") => "L",
            _ => "M",
        }
    }
    /// Everything on the prerequisites tab. The chart is only laid out again when
    /// it has to be (first view, watched list changed, "Hide watched" flipped);
    /// picking a title just re-lights the existing chart so nothing moves.
    fn render_flow(&mut self) {
        let d = data();
        let dm = idx("DM");
        let mut quick: Vec<(usize, &str)> = vec![(dm, "Avengers: Doomsday")];
        if let Some(nx) = self.next_up() {
            quick.insert(0, (nx, "My next title"));
        }
        set_html(
            "quick",
            &quick
                .iter()
                .filter(|(k, _)| !(self.sel_on && *k == self.tg))
                .map(|(k, l)| format!(r##"<button class="chip" data-k="{k}">{l}</button>"##))
                .collect::<String>(),
        );
        self.render_summary();
        let mode = self.pv_mode().to_string();
        attr(&el("pvL"), "aria-pressed", if mode == "L" { "true" } else { "false" });
        attr(&el("pvM"), "aria-pressed", if mode == "M" { "true" } else { "false" });
        cls(&el("routeWrap"), "hide", mode != "L");
        cls(&el("mapWrap"), "hide", mode != "M");
        cls(&el("mapHide"), "hide", mode != "M");
        self.sync_chips();
        place_thumb(&el("pvSeg"));
        // running order: every title that leads in, watched ones stay ticked in place
        if !self.sel_on {
            set_html("route", r#"<li class="already">Pick a title in the chart (or use the shortcuts above) to get its running order.</li>"#);
        } else {
            let t = self.tg;
            let s = anc(t);
            let mut keys: Vec<usize> = s.keys().copied().collect();
            keys.sort();
            let mut h = String::new();
            for &k in &keys {
                let must = s[&k] == 2;
                h += &self.row_html(k, 0, Some(format!(r##"<span class="tag lbl {}">{}</span>"##, if must { "must" } else { "should" }, if must { "Must" } else { "Should" })));
            }
            let tx = &d[t];
            h += &format!(
                r##"<li class="sc target"><span class="then lbl">Then</span><div><div class="t-line"><span class="t-title">{nm}</span><span class="lead"></span><span class="rt">{rt}</span></div><div class="t-meta"><span>{d}</span><span>{st}</span></div></div><span></span></li>"##,
                nm = esc(short(tx.n)), rt = rt(tx), d = tx.d, st = strand(tx.t)
            );
            set_html("route", &h);
        }
        if mode == "M" {
            let stale = match &self.graph {
                None => true,
                Some(g) => self.sel_on && !g.p.contains_key(&self.tg),
            };
            if stale {
                self.render_map();
            }
            self.highlight();
        }
    }
    /// The sticky bar: what's selected and how much of it is left.
    fn render_summary(&self) {
        let d = data();
        cls(&el("selClear"), "hide", !self.sel_on);
        if !self.sel_on {
            el("selName").set_text_content(Some("Nothing selected"));
            set_html("verdict", "Tap a title to light up what you need to watch before it.");
            cls(&el("split"), "off", true);
            return;
        }
        let t = self.tg;
        let s = anc(t);
        let (mut m, mut sm, mut cm, mut cs, mut seen) = (0u32, 0u32, 0usize, 0usize, 0usize);
        for (&k, &w) in &s {
            if self.dn(k) {
                seen += 1;
                continue;
            }
            if w == 2 {
                m += d[k].m;
                cm += 1;
            } else {
                sm += d[k].m;
                cs += 1;
            }
        }
        el("selName").set_text_content(Some(short(d[t].n)));
        let verdict = if s.is_empty() {
            "Stands on its own. Nothing to watch first.".to_string()
        } else if cm + cs == 0 {
            format!("You've seen all {} titles that lead in. Go watch it.", s.len())
        } else {
            let mut parts = vec![];
            if cm > 0 {
                parts.push(format!("<b>{cm} must</b> {}", fmt(m)));
            }
            if cs > 0 {
                parts.push(format!("<b>{cs} should</b> {}", fmt(sm)));
            }
            parts.push(format!("<b>{}</b> to go", fmt(m + sm)));
            if seen > 0 {
                parts.push(format!("{seen} watched"));
            }
            parts.join(" · ")
        };
        set_html("verdict", &verdict);
        let tot = (m + sm).max(1) as f64;
        cls(&el("split"), "off", cm + cs == 0);
        let bars = qa("#split i");
        if bars.len() == 2 {
            attr(&bars[0], "style", &format!("width:{}%", m as f64 / tot * 100.0));
            attr(&bars[1], "style", &format!("width:{}%", sm as f64 / tot * 100.0));
        }
    }
    /// Lay out the whole MCU as one chart: each title sits one step below the
    /// last thing it depends on, ordered to keep lines from crossing.
    fn render_map(&mut self) {
        let d = data();
        let hide = self.st.map_hide;
        let done: HashSet<usize> = self.done().iter().copied().collect();
        let keep = if self.sel_on { Some(self.tg) } else { None };
        let show = |k: usize| !(hide && done.contains(&k)) || Some(k) == keep;
        let v: Vec<usize> = (0..d.len()).filter(|&k| show(k)).collect();

        // nearest visible parents, skipping over hidden (watched) titles
        fn vp(k: usize, show: &dyn Fn(usize) -> bool, memo: &mut HashMap<usize, HashMap<usize, u8>>) -> HashMap<usize, u8> {
            if let Some(m) = memo.get(&k) {
                return m.clone();
            }
            let mut out: HashMap<usize, u8> = HashMap::new();
            for (a, w) in edges(k) {
                if show(a) {
                    let e = out.entry(a).or_insert(0);
                    *e = (*e).max(w);
                } else {
                    for (b, bw) in vp(a, show, memo) {
                        let e = out.entry(b).or_insert(0);
                        *e = (*e).max(w.min(bw));
                    }
                }
            }
            memo.insert(k, out.clone());
            out
        }
        let mut memo = HashMap::new();
        let mut p: HashMap<usize, Vec<(usize, u8)>> = HashMap::new();
        let mut c: HashMap<usize, Vec<usize>> = v.iter().map(|&k| (k, vec![])).collect();
        for &k in &v {
            let ps = vp(k, &show, &mut memo);
            let ks: Vec<usize> = ps.keys().copied().collect();
            let mut list: Vec<(usize, u8)> = ks
                .iter()
                .copied()
                .filter(|&a| !ks.iter().any(|&b| b != a && reach(b).contains(&a)))
                .map(|a| (a, ps[&a]))
                .collect();
            list.sort();
            for &(a, _) in &list {
                c.entry(a).or_default().push(k);
            }
            p.insert(k, list);
        }
        fn lvl(k: usize, p: &HashMap<usize, Vec<(usize, u8)>>, memo: &mut HashMap<usize, usize>) -> usize {
            if let Some(&l) = memo.get(&k) {
                return l;
            }
            let l = p[&k].iter().map(|&(a, _)| lvl(a, p, memo) + 1).max().unwrap_or(0);
            memo.insert(k, l);
            l
        }
        let mut lv: HashMap<usize, usize> = HashMap::new();
        for &k in &v {
            lvl(k, &p, &mut lv);
        }
        let top = v.iter().map(|k| lv[k]).max().unwrap_or(0);
        let mut rows: Vec<Vec<usize>> = (0..=top)
            .map(|l| v.iter().copied().filter(|k| lv[k] == l).collect::<Vec<_>>())
            .filter(|r| !r.is_empty())
            .collect();
        // barycentre sweeps to cut crossings
        let mut pos: HashMap<usize, f64> = HashMap::new();
        let set_pos = |rows: &Vec<Vec<usize>>, pos: &mut HashMap<usize, f64>| {
            for r in rows {
                let n = r.len();
                for (j, &k) in r.iter().enumerate() {
                    pos.insert(k, if n > 1 { j as f64 / (n - 1) as f64 } else { 0.5 });
                }
            }
        };
        set_pos(&rows, &mut pos);
        let bary = |k: usize, list: &[usize], pos: &HashMap<usize, f64>| -> f64 {
            if list.is_empty() { pos[&k] } else { list.iter().map(|b| pos[b]).sum::<f64>() / list.len() as f64 }
        };
        for pass in 0..4 {
            let order: Vec<usize> = if pass % 2 == 0 { (1..rows.len()).collect() } else { (0..rows.len().saturating_sub(1)).rev().collect() };
            for l in order {
                let mut keyed: Vec<(f64, usize)> = rows[l]
                    .iter()
                    .map(|&k| {
                        let list: Vec<usize> = if pass % 2 == 0 { p[&k].iter().map(|x| x.0).collect() } else { c[&k].clone() };
                        (bary(k, &list, &pos), k)
                    })
                    .collect();
                keyed.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal).then(a.1.cmp(&b.1)));
                rows[l] = keyed.into_iter().map(|x| x.1).collect();
                set_pos(&rows, &mut pos);
            }
        }
        let nx = self.next_up();
        let mut h = String::new();
        for (n, r) in rows.iter().enumerate() {
            h += &format!(r##"<div class="band"><div class="step lbl">Step<b>{}</b></div><div class="lvl">"##, n + 1);
            for &k in r {
                let x = &d[k];
                let dk = done.contains(&k);
                let corner = if dk {
                    format!(r##"<span class="ok">{CK}</span>"##)
                } else if let Some(b) = badge(x.t) {
                    format!(r##"<span class="bdg lbl {}">{}</span>"##, x.t, b)
                } else {
                    String::new()
                };
                h += &format!(
                    r##"<button class="nd{dn}{nx}" id="n_{k}" data-k="{k}" aria-pressed="false" aria-label="{n}{w}. Show what to watch before it"><span class="n-t">{sn}</span><span class="n-m">{d}{m}</span>{corner}</button>"##,
                    dn = if dk { " done" } else { "" }, nx = if Some(k) == nx { " nx" } else { "" }, k = k, n = esc(x.n),
                    w = if dk { ", watched" } else { "" }, sn = esc(short(x.n)), d = x.d,
                    m = if x.m > 0 { format!(" · {}", rt(x)) } else { String::new() }, corner = corner
                );
            }
            h += "</div></div>";
        }
        set_html("lv", &h);
        let mut edges_v = vec![];
        for &k in &v {
            for &(a, w) in &p[&k] {
                edges_v.push((a, k, w));
            }
        }
        self.graph = Some(Graph { p, edges: edges_v });
        self.draw();
    }
    /// Light up the selected title and everything that leads into it; fade the rest.
    fn highlight(&self) {
        let fc = el("fc");
        cls(&fc, "sel", self.sel_on);
        let s = if self.sel_on { anc(self.tg) } else { HashMap::new() };
        let need = |k: usize| self.sel_on && (k == self.tg || s.contains_key(&k));
        for n in qa("#lv .nd") {
            let Some(k) = num_attr(&n, "data-k") else { continue };
            let pick = self.sel_on && k == self.tg;
            cls(&n, "pick", pick);
            cls(&n, "need", need(k));
            cls(&n, "must", !pick && s.get(&k) == Some(&2));
            cls(&n, "should", !pick && s.get(&k) == Some(&1));
            attr(&n, "aria-pressed", if pick { "true" } else { "false" });
        }
        for e in qa("#sv path.e") {
            let a = num_attr(&e, "data-a").unwrap_or(usize::MAX);
            let b = num_attr(&e, "data-k").unwrap_or(usize::MAX);
            cls(&e, "lit", need(a) && need(b));
        }
    }
    fn draw(&self) {
        let Some(g) = &self.graph else { return };
        if has_cls("mapWrap", "hide") || has_cls("vF", "hide") {
            return;
        }
        let fc = el("fc");
        let o = fc.get_bounding_client_rect();
        let (bl, bt) = (fc.client_left() as f64, fc.client_top() as f64);
        let sv = el("sv");
        attr(&sv, "width", &o.width().to_string());
        attr(&sv, "height", &o.height().to_string());
        // (left, top, width, height, centre-x)
        let mut rc: HashMap<usize, (f64, f64, f64, f64, f64)> = HashMap::new();
        for n in qa("#lv .nd") {
            if let Some(k) = num_attr(&n, "data-k") {
                let r = n.get_bounding_client_rect();
                rc.insert(k, (r.left() - o.left() - bl, r.top() - o.top() - bt, r.width(), r.height(), r.left() - o.left() + r.width() / 2.0));
            }
        }
        let mut outs: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut ins: HashMap<usize, Vec<usize>> = HashMap::new();
        for (ei, e) in g.edges.iter().enumerate() {
            outs.entry(e.0).or_default().push(ei);
            ins.entry(e.1).or_default().push(ei);
        }
        let cx = |k: usize| rc.get(&k).map_or(0.0, |r| r.4);
        for list in outs.values_mut() {
            list.sort_by(|a, b| cx(g.edges[*a].1).partial_cmp(&cx(g.edges[*b].1)).unwrap_or(std::cmp::Ordering::Equal));
        }
        for list in ins.values_mut() {
            list.sort_by(|a, b| cx(g.edges[*a].0).partial_cmp(&cx(g.edges[*b].0)).unwrap_or(std::cmp::Ordering::Equal));
        }
        let port = |r: &(f64, f64, f64, f64, f64), i: usize, n: usize| r.0 + r.2 * (0.18 + 0.64 * (i as f64 + 1.0) / (n as f64 + 1.0));
        let mut svg = String::from(r##"<defs><marker id="mk-m" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="6" markerHeight="6" orient="auto"><path class="mk-m" d="M0 0L10 5L0 10z"/></marker><marker id="mk-s" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="6" markerHeight="6" orient="auto"><path class="mk-s" d="M0 0L10 5L0 10z"/></marker></defs>"##);
        for (ei, &(a, k, w)) in g.edges.iter().enumerate() {
            let (Some(ra), Some(rk)) = (rc.get(&a), rc.get(&k)) else { continue };
            let oa = &outs[&a];
            let ik = &ins[&k];
            let x1 = port(ra, oa.iter().position(|&x| x == ei).unwrap_or(0), oa.len());
            let y1 = ra.1 + ra.3;
            let x2 = port(rk, ik.iter().position(|&x| x == ei).unwrap_or(0), ik.len());
            let y2 = rk.1 - 2.0;
            let dy = ((y2 - y1) * 0.5).max(26.0);
            svg += &format!(
                r##"<path class="e {c}" data-a="{a}" data-k="{k}" marker-end="url(#mk-{m})" d="M{x1:.1},{y1:.1}C{x1:.1},{c1:.1} {x2:.1},{c2:.1} {x2:.1},{y2:.1}"/>"##,
                c = if w == 2 { "e-must" } else { "e-should" }, m = if w == 2 { "m" } else { "s" }, a = a, k = k,
                x1 = x1, y1 = y1, c1 = y1 + dy, x2 = x2, c2 = y2 - dy, y2 = y2
            );
        }
        sv.set_inner_html(&svg);
    }
    // ---------------- tabs ----------------
    fn show(&mut self, v: &str) {
        self.st.view = v.into();
        self.save();
        cls(&el("vL"), "hide", v != "L");
        cls(&el("vF"), "hide", v != "F");
        attr(&el("tL"), "aria-selected", if v == "L" { "true" } else { "false" });
        attr(&el("tF"), "aria-selected", if v == "F" { "true" } else { "false" });
        place_thumb(&el("tabs"));
        if v == "F" {
            self.render_flow();
        } else {
            self.render_list();
        }
    }
}

// ---------------- free helpers that don't need state ----------------

fn nx_title(c: &Ctx) -> String {
    c.nx.map(|n| short(data()[n].n).to_string()).unwrap_or_else(|| "See you at Doomsday".into())
}
fn nx_meta(c: &Ctx) -> String {
    match c.nx {
        Some(n) => {
            let x = &data()[n];
            format!("<span>{}</span><span>{}{}</span><span>{}</span>", x.d, strand(x.t), paren(x.n).map(|p| format!(" · {p}")).unwrap_or_default(), rt(x))
        }
        None => "<span>18 December 2026</span><span>In cinemas</span>".into(),
    }
}
fn acts(c: &Ctx, dark: &str, line: &str) -> String {
    match c.nx {
        Some(n) => format!(
            r##"<div class="acts"><button class="btn {dark} lbl" data-act="done" data-i="{n}">{ck}Mark watched</button><button class="btn {line} lbl" data-act="pre" data-k="{n}">{tree}Watch first</button></div>"##,
            ck = ico("check"), tree = ico("tree")
        ),
        None => format!(r##"<div class="acts"><button class="btn {line} lbl" data-act="pre" data-k="{}">{}See Doomsday's chain</button></div>"##, idx("DM"), ico("tree")),
    }
}
fn pre_btn(i: usize, label: &str) -> String {
    format!(
        r##"<button class="pre lbl" data-k="{i}" aria-label="What to watch before {n}"><span>{label}</span>{a}</button>"##,
        n = esc(data()[i].n), a = ico("arrow")
    )
}
fn place_thumb(seg: &Element) {
    let on = seg.query_selector(r#"[aria-selected="true"],[aria-pressed="true"]"#).ok().flatten();
    let th = seg.query_selector(".thumb").ok().flatten();
    if let (Some(on), Some(th)) = (on, th) {
        let on: HtmlElement = on.unchecked_into();
        attr(&th, "style", &format!("width:{}px;transform:translateX({}px)", on.offset_width(), on.offset_left()));
    }
}
fn thumbs() {
    for s in qa(".seg") {
        place_thumb(&s);
    }
}
fn look_open() {
    set_hidden("lookPop", false);
    attr(&el("lookBtn"), "aria-expanded", "true");
    if let Ok(Some(c)) = el("lookPop").query_selector(r#"[aria-checked="true"]"#) {
        focus(&c);
    }
}
fn look_close(refocus: bool) {
    if is_hidden("lookPop") {
        return;
    }
    set_hidden("lookPop", true);
    attr(&el("lookBtn"), "aria-expanded", "false");
    if refocus {
        focus(&el("lookBtn"));
    }
}
fn has_selection() -> bool {
    win().get_selection().ok().flatten().map(|s| String::from(s.to_string()).len() > 0).unwrap_or(false)
}

// =====================================================================
// Startup: wire every event, then render
// =====================================================================

#[wasm_bindgen(start)]
pub fn start() {
    // look picker
    on_id("lookBtn", "click", |_| if is_hidden("lookPop") { look_open() } else { look_close(false) });
    on_id("lookPop", "click", |e| {
        if let Some(o) = closest(&e, ".look-opt") {
            let id = o.get_attribute("data-id").unwrap_or_default();
            with(|a| {
                a.apply_look(&id);
                a.save();
            });
            look_close(true);
        }
    });
    on_id("lookPop", "keydown", |e| {
        let opts = qa("#lookPop .look-opt");
        if opts.is_empty() {
            return;
        }
        let active = doc().active_element();
        let n = opts.iter().position(|o| Some(o) == active.as_ref()).unwrap_or(0);
        match key_of(&e).as_str() {
            "ArrowDown" => { e.prevent_default(); focus(&opts[(n + 1) % opts.len()]); }
            "ArrowUp" => { e.prevent_default(); focus(&opts[(n + opts.len() - 1) % opts.len()]); }
            "Escape" => { e.prevent_default(); look_close(true); }
            "Tab" => look_close(false),
            _ => {}
        }
    });

    // hero actions
    on_id("hero", "click", |e| {
        let Some(a) = closest(&e, "[data-act],[data-jump]") else { return };
        with(|app| {
            if let Some(j) = num_attr(&a, "data-jump") {
                return app.scroll_to_row(j);
            }
            match a.get_attribute("data-act").as_deref() {
                Some("done") => {
                    if let Some(i) = num_attr(&a, "data-i") {
                        app.toggle(i);
                        app.refresh();
                    }
                }
                Some("pre") => {
                    if let Some(k) = num_attr(&a, "data-k") {
                        app.plan_for(k);
                    }
                }
                Some("jumpq") => {
                    if let Some(i) = num_attr(&a, "data-i") {
                        app.scroll_to_row(i);
                    }
                }
                _ => {}
            }
        });
    });

    // watchlist
    on_id("list", "click", |e| {
        if let Some(b) = closest(&e, ".pre") {
            if let Some(k) = num_attr(&b, "data-k") {
                with(|a| a.plan_for(k));
            }
            return;
        }
        let Some(r) = closest(&e, "[data-i]") else { return };
        if has_selection() {
            return;
        }
        let Some(i) = num_attr(&r, "data-i") else { return };
        with(|a| {
            a.toggle(i);
            a.render_top();
            a.mark_next();
            if a.st.hide_s && a.dn(i) {
                timeout(if reduced() { 0 } else { 650 }, || {
                    with(|a| a.render_list());
                });
            }
        });
    });
    on_id("jump", "click", |_| {
        with(|a| {
            if let Some(n) = a.next_up() {
                a.scroll_to_row(n);
            }
        });
    });
    on_id("incN", "click", |_| {
        with(|a| {
            a.st.skip_n = !a.st.skip_n;
            a.sync_chips();
            a.save();
            a.render_top();
            a.render_list();
        });
    });
    on_id("hideS", "click", |_| {
        with(|a| {
            a.st.hide_s = !a.st.hide_s;
            a.sync_chips();
            a.save();
            a.render_list();
        });
    });

    on(&doc(), "pointerdown", |e| {
        let t: Option<web_sys::Node> = e.target().and_then(|t| t.dyn_into().ok());
        if !is_hidden("lookPop") && !el("look").contains(t.as_ref()) {
            look_close(false);
        }
    });

    // prerequisites: tap a title in the chart to light up what leads into it
    on_id("lv", "click", |e| {
        if let Some(k) = closest(&e, ".nd").and_then(|n| num_attr(&n, "data-k")) {
            with(|a| a.select(Some(k)));
        }
    });
    on_id("selClear", "click", |_| {
        with(|a| a.select(None));
    });

    // prerequisites: running order + controls
    on_id("route", "click", |e| {
        if let Some(b) = closest(&e, ".pre") {
            if let Some(k) = num_attr(&b, "data-k") {
                with(|a| {
                    a.sel_on = false;
                    a.select(Some(k));
                });
            }
            return;
        }
        let Some(r) = closest(&e, ".sc") else { return };
        if r.class_list().contains("target") {
            return;
        }
        let Some(i) = num_attr(&r, "data-i") else { return };
        with(|a| {
            a.toggle(i); // saved straight away
            a.render_top();
            let d = a.dn(i);
            cls(&r, "seen", d);
            if let Ok(Some(p)) = r.query_selector(".punch") {
                attr(&p, "aria-pressed", if d { "true" } else { "false" });
                attr(&p, "aria-label", &format!("{} {} watched", if d { "Unmark" } else { "Mark" }, data()[i].n));
            }
            a.render_summary();
        });
    });
    on_id("quick", "click", |e| {
        if let Some(k) = closest(&e, "[data-k]").and_then(|b| num_attr(&b, "data-k")) {
            with(|a| {
                a.sel_on = false;
                a.select(Some(k));
                if let Some(n) = doc().get_element_by_id(&format!("n_{k}")).filter(|_| !has_cls("mapWrap", "hide")) {
                    into_view(&n, ScrollLogicalPosition::Center, true);
                }
            });
        }
    });
    on_id("pvL", "click", |_| {
        with(|a| {
            a.st.pv = Some("L".into());
            a.save();
            a.render_flow();
        });
    });
    on_id("pvM", "click", |_| {
        with(|a| {
            a.st.pv = Some("M".into());
            a.save();
            a.render_flow();
        });
    });
    on_id("mapHide", "click", |_| {
        with(|a| {
            a.st.map_hide = !a.st.map_hide;
            a.graph = None;
            a.save();
            a.render_flow();
        });
    });

    // keep the map lines attached when anything resizes
    let ro_cb = Closure::<dyn FnMut(JsValue)>::new(|_: JsValue| {
        with(|a| a.draw());
    });
    if let Ok(ro) = web_sys::ResizeObserver::new(ro_cb.as_ref().unchecked_ref()) {
        ro.observe(&el("fc"));
        std::mem::forget(ro);
    }
    ro_cb.forget();
    on(&win(), "resize", |_| {
        with(|a| a.draw());
        thumbs();
    });

    // tabs
    on_id("tL", "click", |_| {
        with(|a| a.show("L"));
    });
    on_id("tF", "click", |_| {
        with(|a| a.show("F"));
    });
    on_id("tabs", "keydown", |e| {
        if matches!(key_of(&e).as_str(), "ArrowRight" | "ArrowLeft" | "ArrowDown" | "ArrowUp") {
            e.prevent_default();
            let v = with(|a| {
                let v = if a.st.view == "L" { "F" } else { "L" };
                a.show(v);
                v
            });
            if let Some(v) = v {
                focus(&el(if v == "L" { "tL" } else { "tF" }));
            }
        }
    });

    // first render
    with(|a| {
        let theme = a.st.theme.clone();
        a.apply_look(&theme);
        a.sync_chips();
        a.render_top();
        let v = if a.st.view == "F" { "F" } else { "L" };
        a.show(v);
        a.booted = true;
    });
    if let Ok(p) = doc().fonts().ready() {
        let cb = Closure::<dyn FnMut(JsValue)>::new(|_: JsValue| {
            thumbs();
            with(|a| a.draw());
        });
        let _ = p.then(&cb);
        cb.forget();
    }
}
