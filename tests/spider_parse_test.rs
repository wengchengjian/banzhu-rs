use banzhu_spider::spider::parse::{
    arr_dup_rem_linked, clean_filename, parse_book_info, parse_chapter_list, parse_section_urls,
};

#[test]
fn test_parse_book_info_extracts_all_fields() {
    let html = std::fs::read_to_string("tests/fixtures/book_detail.html")
        .expect("fixture 文件应存在");
    let book = parse_book_info(12345, &html).expect("parse_book_info should succeed");

    assert_eq!(book.title, "测试书名");
    assert_eq!(book.author, "测试作者");
    assert_eq!(book.category, "玄幻");
    assert_eq!(book.page, 5);
    assert_eq!(book.count, 100000);
    assert_eq!(book.likes, 200);
    assert_eq!(book.introduce, "这是一本测试书的简介内容");
    assert_eq!(book.filename, "测试书名");
    // 保留原 get_info 行为：Book.id 设为 0，参数 book_id 仅用于错误信息
    assert_eq!(book.id, 0);
    assert_eq!(book.num, 0);
}

#[test]
fn test_parse_book_info_missing_pagination_returns_error() {
    let html = "<html><body><h1>x</h1></body></html>";
    let err = parse_book_info(99, html).unwrap_err();
    assert!(err.to_string().contains("book:99"), "错误信息应包含 book_id: {}", err);
}

#[test]
fn test_clean_filename_replaces_illegal_chars() {
    let cleaned = clean_filename("书名/测试:1.txt");
    assert_eq!(cleaned, "书名_测试_1.txt");
}

#[test]
fn test_clean_filename_replaces_all_illegal_chars() {
    let cleaned = clean_filename(r#"a\b/c*d?e"f<g>h|i"#);
    assert_eq!(cleaned, "a_b_c_d_e_f_g_h_i");
}

#[test]
fn test_clean_filename_keeps_legal_name() {
    assert_eq!(clean_filename("正常书名"), "正常书名");
}

#[test]
fn test_parse_chapter_list_returns_chapters() {
    let html = std::fs::read_to_string("tests/fixtures/chapter_page.html").unwrap();
    let chapters = parse_chapter_list(&html, "https://www.bz555555555.com").unwrap();
    assert_eq!(chapters.len(), 3);
    let first = &chapters[0];
    assert_eq!(first.title, "第1章 开始");
    assert_eq!(first.url, "https://www.bz555555555.com/12/12345_1/23456.html");
}

#[test]
fn test_parse_section_urls_returns_sections() {
    // chapter URL 形如 https://www.bz555555555.com/12/12345_1/23456.html
    let chapter_url = "https://www.bz555555555.com/12/12345_1/23456.html";
    let html = std::fs::read_to_string("tests/fixtures/chapter_page.html").unwrap();
    let sections = parse_section_urls(chapter_url, &html).unwrap();
    assert_eq!(sections.len(), 3);
    assert_eq!(sections[0].url, "https://www.bz555555555.com/12/12345_1/23456_1.html");
    assert_eq!(sections[2].url, "https://www.bz555555555.com/12/12345_1/23456_3.html");
}

#[test]
fn test_arr_dup_rem_linked_removes_duplicates() {
    let input = vec![1, 2, 2, 3, 3, 3, 4];
    let result = arr_dup_rem_linked(input);
    assert_eq!(result, vec![1, 2, 3, 4]);
}
