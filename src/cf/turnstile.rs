use headless_chrome::protocol::cdp::DOM::{GetContentQuads, GetDocument, Node};
use headless_chrome::protocol::cdp::Input::{
    DispatchMouseEvent, DispatchMouseEventTypeOption, MouseButton,
};
use headless_chrome::Tab;
use log::debug;
use std::sync::Arc;
use std::time::Duration;

/// 尝试找到并点击 Turnstile 验证框
///
/// Turnstile iframe 在 closed shadow DOM 内，普通 JS 无法访问。
/// 必须用 CDP DOM.getDocument(pierce=true) 穿透，再用 GetContentQuads 取坐标。
pub(crate) fn try_click_turnstile(tab: &Arc<Tab>, round: u32) -> bool {
    // Step 1: CDP 穿透 shadow DOM 获取完整 DOM 树
    let doc = match tab.call_method(GetDocument {
        depth: Some(200),
        pierce: Some(true),
    }) {
        Ok(r) => r,
        Err(_) => return false,
    };

    // Step 2: 递归查找 Turnstile iframe
    let iframe_id = match find_turnstile_iframe(&doc.root) {
        Some(id) => id,
        None => return false,
    };

    // Step 3: 获取 iframe 的视口坐标
    let quads = match tab.call_method(GetContentQuads {
        node_id: Some(iframe_id),
        backend_node_id: None,
        object_id: None,
    }) {
        Ok(r) => r,
        Err(_) => return false,
    };

    let quad = match quads.quads.first() {
        Some(q) if q.len() >= 8 => q,
        _ => return false,
    };

    let iframe_x = quad[0];
    let iframe_y = quad[1];
    let iframe_h = quad[5] - quad[1];

    // Turnstile checkbox 在 iframe 左侧约 32px，垂直居中
    let cx = iframe_x + 32.0 + (round as f64 % 5.0 - 2.0) * 3.0;
    let cy = iframe_y + iframe_h / 2.0 + (round as f64 % 3.0 - 1.0) * 2.0;

    if round <= 3 {
        debug!(
            "[click #{}] Turnstile iframe nodeId={}, ({:.0},{:.0}), clicking ({:.0},{:.0})",
            round, iframe_id, iframe_x, iframe_y, cx, cy
        );
    }

    // Step 4: 模拟鼠标移动（ease-out 减速）
    let steps = 10;
    let sx = cx - 50.0 + (round as f64 % 7.0 - 3.0) * 15.0;
    let sy = cy - 40.0 + (round as f64 % 5.0 - 2.0) * 12.0;

    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let ease = 1.0 - (1.0 - t) * (1.0 - t);
        let mx = sx + (cx - sx) * ease;
        let my = sy + (cy - sy) * ease;

        let _ = tab.call_method(DispatchMouseEvent {
            Type: DispatchMouseEventTypeOption::MouseMoved,
            x: mx,
            y: my,
            modifiers: Some(0),
            timestamp: None,
            button: Some(MouseButton::None),
            buttons: Some(0),
            click_count: None,
            delta_x: None,
            delta_y: None,
            pointer_Type: None,
            force: None,
            tangential_pressure: None,
            tilt_x: None,
            tilt_y: None,
            twist: None,
        });
        std::thread::sleep(Duration::from_millis(10 + (i as u64 * 5).min(40)));
    }

    // Step 5: 点击
    let _ = tab.call_method(DispatchMouseEvent {
        Type: DispatchMouseEventTypeOption::MousePressed,
        x: cx,
        y: cy,
        modifiers: Some(0),
        timestamp: None,
        button: Some(MouseButton::Left),
        buttons: Some(0),
        click_count: Some(1),
        delta_x: None,
        delta_y: None,
        pointer_Type: None,
        force: None,
        tangential_pressure: None,
        tilt_x: None,
        tilt_y: None,
        twist: None,
    });
    std::thread::sleep(Duration::from_millis(60 + (round as u64 * 13) % 50));
    let _ = tab.call_method(DispatchMouseEvent {
        Type: DispatchMouseEventTypeOption::MouseReleased,
        x: cx,
        y: cy,
        modifiers: Some(0),
        timestamp: None,
        button: Some(MouseButton::Left),
        buttons: Some(0),
        click_count: Some(1),
        delta_x: None,
        delta_y: None,
        pointer_Type: None,
        force: None,
        tangential_pressure: None,
        tilt_x: None,
        tilt_y: None,
        twist: None,
    });

    true
}

/// 递归遍历 DOM 树（含 shadow DOM），找 Turnstile iframe 的 nodeId
fn find_turnstile_iframe(node: &Node) -> Option<u32> {
    let attrs = node.attributes.as_deref().unwrap_or(&[]);
    let tag = node.node_name.to_lowercase();

    if tag == "iframe" {
        let is_turnstile = attrs.chunks(2).any(|pair| {
            pair.len() == 2
                && ((pair[0] == "src" && pair[1].contains("challenges.cloudflare.com"))
                    || (pair[0] == "id" && pair[1].contains("cf-chl-widget")))
        });
        if is_turnstile {
            return Some(node.node_id);
        }
    }

    // 递归子节点
    if let Some(children) = node.children.as_deref() {
        for child in children {
            if let Some(id) = find_turnstile_iframe(child) {
                return Some(id);
            }
        }
    }

    // 递归 shadow roots（穿透 closed shadow DOM）
    if let Some(shadow_roots) = node.shadow_roots.as_deref() {
        for sr in shadow_roots {
            if let Some(sr_children) = sr.children.as_deref() {
                for sr_child in sr_children {
                    if let Some(id) = find_turnstile_iframe(sr_child) {
                        return Some(id);
                    }
                }
            }
        }
    }

    // 递归 iframe contentDocument
    if let Some(ref doc) = node.content_document {
        if let Some(id) = find_turnstile_iframe(doc) {
            return Some(id);
        }
    }

    None
}
