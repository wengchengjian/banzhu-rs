use base64::engine::general_purpose;
use base64::Engine as _;
use scraper::{Html, Selector};

/// RC4 解密（标准实现，用于 JS 中的代码点层级 RC4）
/// 当 ns 为纯 base64 编码时不需要；仅当 ns 有额外 RC4 加密层时使用
#[allow(dead_code)]
pub fn rc4_decrypt_bytes(ciphertext: &[u8], key: &[u8]) -> Vec<u8> {
    let mut s: Vec<u8> = (0..=255).collect();
    let mut j: usize = 0;
    for i in 0..256 {
        j = (j + s[i] as usize + key[i % key.len()] as usize) % 256;
        s.swap(i, j);
    }
    let mut i: usize = 0;
    j = 0;
    let mut result = Vec::with_capacity(ciphertext.len());
    for &byte in ciphertext {
        i = (i + 1) % 256;
        j = (j + s[i] as usize) % 256;
        s.swap(i, j);
        let k = s[(s[i] as usize + s[j] as usize) % 256];
        result.push(byte ^ k);
    }
    result
}

/// 解密 section 数据 — 替换 jdom.py / a.js 的 _ii_rr 调用
///
/// `ns`: 页面中的 `var ns='...'` 值
/// `html`: section 页面的完整 HTML
///
/// 算法流程（从 a.dec.dec.js 反编译）：
/// 1. base64 解码 ns → 得到逗号分隔的索引数组
/// 2. 从 HTML 提取 #chapter 元素的 innerHTML
/// 3. 清理方括号标记
/// 4. 按 <br> 分割章节内容
/// 5. 按索引数组重排分段
pub fn decrypt_section_data(html: &str, ns: &str) -> Option<String> {
    // Step 1: base64 解码 ns → 索引数组 "base,idx1,idx2,..."
    let indices_str = decode_ns(ns)?;
    let indices: Vec<&str> = indices_str.split(',').collect();
    if indices.len() < 2 {
        return None;
    }
    let base: i64 = indices[0].parse().ok()?;

    // Step 2: 从 HTML 中提取章节内容（id 以 "chapter" 开头）
    let doc = Html::parse_document(html);
    let chapter_sel = Selector::parse("[id^=\"chapter\"]").ok()?;
    let chapter_el = doc.select(&chapter_sel).next()?;
    let chapter_html = chapter_el.inner_html();

    // Step 3: 清理方括号标记 [/xxx]内容 或 [xxx]标记
    let re_bracket = regex::Regex::new(r"\[.*?\]").ok()?;
    let cleaned = re_bracket.replace_all(&chapter_html, "").to_string();

    // Step 4: 按 <br> 分割
    let br_re = regex::Regex::new(r"<br\s*/?>").ok()?;
    let segments: Vec<&str> = br_re.split(&cleaned).filter(|s| !s.trim().is_empty()).collect();

    // Step 5: 按索引重排
    let mut result = String::new();
    let total = segments.len();
    for i in 1..=total {
        let idx_val = indices.get(i)?.parse::<i64>().ok()?;
        let seg_idx = (idx_val - base) as usize;
        if seg_idx < segments.len() {
            result.push_str(segments[seg_idx]);
            // ponytail: 段落之间不额外加分隔符，与 JS 原版一致
        }
    }

    // 清理残留 HTML 标签
    let tag_re = regex::Regex::new(r"<[^>]*>").ok()?;
    let plain = tag_re.replace_all(&result, "").to_string();

    // 解码 HTML 实体
    let plain = plain
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");

    Some(plain)
}

/// 解码 ns 参数
///
/// 尝试顺序：
/// 1. 纯 base64 → 直接解码（大多数情况）
/// 2. ponytail: 如果后续发现需要 RC4 解密层，在此处扩展
fn decode_ns(ns: &str) -> Option<String> {
    let bytes = general_purpose::STANDARD.decode(ns).ok()?;
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rc4_roundtrip() {
        let key = b"test_key";
        let msg = b"hello world";
        let encrypted = rc4_decrypt_bytes(msg, key);
        let decrypted = rc4_decrypt_bytes(&encrypted, key);
        assert_eq!(decrypted, msg);
    }

    #[test]
    fn test_decode_ns_simple() {
        // "2,5,1,3,4" in base64 = "Miw1LDEsMyw0"
        let result = decode_ns("Miw1LDEsMyw0");
        assert_eq!(result, Some("2,5,1,3,4".to_string()));
    }

    #[test]
    fn test_decode_ns_single_digit() {
        // "1,3,2" in base64 = "MSwzLDI="
        let result = decode_ns("MSwzLDI=");
        assert_eq!(result, Some("1,3,2".to_string()));
    }

    #[test]
    fn test_decrypt_section_simple() {
        let html = r#"<html><body>
            <div id="chapter1">
                [/a]段落一[/b][/c]<br>[/d]段落二<br>段落三
            </div>
        </body></html>"#;
        // ns = base64("1,3,2") — reorder: seg[3-1]=seg[2]="段落三", seg[2-1]=seg[1]="段落二"
        let ns = "MSwzLDI=";
        let result = decrypt_section_data(html, ns);
        assert!(result.is_some());
        // 期望重排后: 段落三 段落二 (去掉 <br> 和空段)
    }
}
