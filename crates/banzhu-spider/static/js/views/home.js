// ─── 首页视图 ────────────────────────────────────────────────────────────────

Views.home = async function(app, params) {
  const currentCategory = params.category || '';
  let currentPage = 1;
  const limit = 20;
  let loading = false;
  let hasMore = true;

  app.innerHTML = `
    ${renderHeader()}
    <main>
      <div class="container">
        <section class="categories-section">
          <div class="category-tags" id="categoryTags">
            <a class="category-tag ${!currentCategory ? 'active' : ''}" href="#/">全部</a>
          </div>
        </section>
        <section class="books-section">
          <h2>书库</h2>
          <div id="bookGrid" class="book-grid"></div>
          <div id="scrollSentinel" class="scroll-sentinel"></div>
          <p id="loadingIndicator" class="empty" style="display:none">加载中...</p>
        </section>
      </div>
    </main>
    <footer class="site-footer">
      <p>版主网 - 简洁无广告小说阅读</p>
    </footer>`;

  bindHeaderEvents();

  // 加载分类
  try {
    const categories = await API.get('/categories');
    const tagsEl = document.getElementById('categoryTags');
    if (categories && categories.length > 0) {
      tagsEl.innerHTML = `<a class="category-tag ${!currentCategory ? 'active' : ''}" href="#/">全部</a>` +
        categories.map(function(c) {
          const name = typeof c === 'string' ? c : c.name || c;
          const active = name === currentCategory ? ' active' : '';
          return `<a class="category-tag${active}" href="#/?category=${encodeURIComponent(name)}">${escapeHtml(name)}</a>`;
        }).join('');
    }
  } catch (e) {
    // 分类加载失败不影响主流程
  }

  // 加载书籍
  const gridEl = document.getElementById('bookGrid');
  const loadingEl = document.getElementById('loadingIndicator');

  async function loadBooks() {
    if (loading || !hasMore) return;
    loading = true;
    loadingEl.style.display = 'block';

    try {
      let path = '/books?page=' + currentPage + '&limit=' + limit;
      if (currentCategory) path += '&category=' + encodeURIComponent(currentCategory);
      const books = await API.get(path);

      if (!books || books.length === 0) {
        hasMore = false;
        if (currentPage === 1) {
          gridEl.innerHTML = '<p class="empty">暂无书籍</p>';
        }
      } else {
        gridEl.insertAdjacentHTML('beforeend', books.map(renderBookCard).join(''));
        if (books.length < limit) hasMore = false;
        currentPage++;
      }
    } catch (e) {
      if (currentPage === 1) {
        gridEl.innerHTML = '<p class="empty">加载失败: ' + escapeHtml(e.message) + '</p>';
      }
      hasMore = false;
    }

    loading = false;
    loadingEl.style.display = 'none';
  }

  await loadBooks();

  // 无限滚动
  const sentinel = document.getElementById('scrollSentinel');
  if (sentinel && 'IntersectionObserver' in window) {
    const observer = new IntersectionObserver(function(entries) {
      if (entries[0].isIntersecting) {
        loadBooks();
      }
    }, { rootMargin: '200px' });
    observer.observe(sentinel);
  }
};
