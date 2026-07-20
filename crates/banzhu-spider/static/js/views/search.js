// ─── 搜索视图 ────────────────────────────────────────────────────────────────

Views.search = async function(app, params) {
  const query = params.q || '';
  const field = params.field || '';

  app.innerHTML = `
    ${renderHeader()}
    <main>
      <div class="container">
        <div class="search-page-header">
          <h1>搜索</h1>
          <form class="search-form" id="searchForm">
            <input type="text" id="searchInput" value="${escapeHtml(query)}" placeholder="输入关键词搜索...">
            <select id="searchField">
              <option value="">全部</option>
              <option value="title" ${field === 'title' ? 'selected' : ''}>书名</option>
              <option value="author" ${field === 'author' ? 'selected' : ''}>作者</option>
              <option value="content" ${field === 'content' ? 'selected' : ''}>正文</option>
            </select>
            <button type="submit">搜索</button>
          </form>
        </div>
        <div id="searchResults"></div>
      </div>
    </main>
    <footer class="site-footer"><p>版主网 - 简洁无广告小说阅读</p></footer>`;

  bindHeaderEvents();

  const resultsEl = document.getElementById('searchResults');
  const searchForm = document.getElementById('searchForm');
  const searchInput = document.getElementById('searchInput');
  const searchField = document.getElementById('searchField');

  // 搜索提交
  searchForm.addEventListener('submit', function(e) {
    e.preventDefault();
    const q = searchInput.value.trim();
    const f = searchField.value;
    let hash = '#/search?q=' + encodeURIComponent(q);
    if (f) hash += '&field=' + f;
    navigate(hash);
  });

  // 执行搜索
  if (query) {
    resultsEl.innerHTML = '<p class="empty">搜索中...</p>';
    try {
      let path = '/search?q=' + encodeURIComponent(query) + '&limit=20';
      if (field) path += '&field=' + encodeURIComponent(field);
      const results = await API.get(path);

      if (!results || results.length === 0) {
        resultsEl.innerHTML = `
          <div class="search-placeholder">
            <p>没有找到与 "<strong>${escapeHtml(query)}</strong>" 相关的结果</p>
            <p class="search-hint">建议：尝试其他关键词，或减少筛选条件</p>
          </div>`;
      } else {
        resultsEl.innerHTML = `
          <p class="search-summary">找到 <strong>${results.length}</strong> 个与 "${escapeHtml(query)}" 相关的结果</p>
          <div class="result-list">${results.map(function(item) {
            return renderSearchResult(item, query);
          }).join('')}</div>`;
      }
    } catch (e) {
      resultsEl.innerHTML = `<div class="search-placeholder"><p>搜索出错: ${escapeHtml(e.message)}</p></div>`;
    }
  } else {
    resultsEl.innerHTML = '<div class="search-placeholder"><p>输入关键词开始搜索</p></div>';
  }
};

function renderSearchResult(item, query) {
  const title = escapeHtml(item.title || '未知');
  const author = escapeHtml(item.author || '佚名');
  const bookId = item.book_id || item.id;

  // 高亮匹配文本
  let snippet = item.snippet || item.content_preview || '';
  if (snippet) {
    snippet = escapeHtml(snippet);
    const escaped = query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    snippet = snippet.replace(new RegExp('(' + escaped + ')', 'gi'), '<mark>$1</mark>');
  }

  return `
  <div class="result-item">
    <h3><a href="#/book/${bookId}">${title}</a></h3>
    <div class="result-meta">
      <span>作者：${author}</span>
      ${item.category ? `<span>分类：${escapeHtml(item.category)}</span>` : ''}
      ${item.word_count ? `<span>${formatWordCount(item.word_count)}</span>` : ''}
    </div>
    ${snippet ? `<div class="result-snippet">${snippet}</div>` : ''}
  </div>`;
}
