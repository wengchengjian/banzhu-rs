// ─── 统计视图 ────────────────────────────────────────────────────────────────

Views.stats = async function(app) {
  app.innerHTML =
    renderHeader() +
    '<main><div class="container">' +
      '<h1 class="page-title">数据统计</h1>' +
      '<div id="statsContent"><p class="empty">加载中...</p></div>' +
    '</div></main>';

  bindHeaderEvents();

  var stats = null;
  var categories = [];

  try {
    var results = await Promise.all([
      API.get('/stats'),
      API.get('/categories').catch(function() { return []; }),
    ]);
    stats = results[0];
    categories = results[1] || [];
  } catch (e) {
    document.getElementById('statsContent').innerHTML = '<p class="empty">加载失败: ' + escapeHtml(e.message) + '</p>';
    return;
  }

  var totalBooks = stats.total_books || stats.book_count || 0;
  var totalChapters = stats.total_chapters || stats.chapter_count || 0;
  var totalWords = stats.total_words || stats.word_count || 0;
  var categoryCount = categories.length || stats.category_count || 0;

  var contentEl = document.getElementById('statsContent');
  contentEl.innerHTML =
    '<div class="stat-cards">' +
      renderStatCard('总书籍', totalBooks) +
      renderStatCard('总章节', totalChapters) +
      renderStatCard('总字数', formatWordCount(totalWords)) +
      renderStatCard('分类数', categoryCount) +
    '</div>' +
    '<section class="stat-section">' +
      '<h2>分类分布</h2>' +
      '<div class="stat-bar-chart" id="statBarChart"></div>' +
    '</section>' +
    '<section class="stat-section">' +
      '<h2>最近收录</h2>' +
      '<div id="recentBooks"><p class="empty">加载中...</p></div>' +
    '</section>';

  // 分类分布图
  renderCategoryChart(categories, totalBooks);

  // 最近收录
  loadRecentBooks();

  function renderStatCard(label, value) {
    return '<div class="stat-card">' +
      '<div class="stat-card-value">' + escapeHtml(String(value)) + '</div>' +
      '<div class="stat-card-label">' + label + '</div>' +
    '</div>';
  }

  function renderCategoryChart(cats, total) {
    var chartEl = document.getElementById('statBarChart');
    if (!cats || cats.length === 0) {
      chartEl.innerHTML = '<p class="empty">暂无分类数据</p>';
      return;
    }

    var barColors = ['#2c5282', '#2f855a', '#9b2c2c', '#744210', '#553c9a', '#285e61', '#975a16', '#1a365d'];
    var maxCount = 0;

    var items = cats.map(function(c, i) {
      var name = typeof c === 'string' ? c : (c.name || '');
      var count = typeof c === 'object' ? (c.count || c.book_count || 0) : 0;
      if (count > maxCount) maxCount = count;
      return { name: name, count: count, color: barColors[i % barColors.length] };
    });

    if (maxCount === 0) maxCount = 1;

    chartEl.innerHTML = items.map(function(item) {
      var percent = Math.round((item.count / maxCount) * 100);
      return '<div class="stat-bar-row">' +
        '<span class="stat-bar-label">' + escapeHtml(item.name) + '</span>' +
        '<div class="stat-bar-track">' +
          '<div class="stat-bar-fill" style="width:' + percent + '%;background:' + item.color + '"></div>' +
        '</div>' +
        '<span class="stat-bar-count">' + item.count + '</span>' +
      '</div>';
    }).join('');
  }

  async function loadRecentBooks() {
    var el = document.getElementById('recentBooks');
    try {
      var books = await API.get('/books?page=1&limit=10');
      if (!books || books.length === 0) {
        el.innerHTML = '<p class="empty">暂无书籍</p>';
        return;
      }
      el.innerHTML = '<div class="stat-recent-list">' + books.map(function(book) {
        return '<div class="stat-recent-item">' +
          '<a href="#/book/' + book.id + '" class="stat-recent-title">' + escapeHtml(book.title) + '</a>' +
          '<span class="stat-recent-meta">' + escapeHtml(book.author || '佚名') + '</span>' +
          (book.created_at ? '<span class="stat-recent-date">' + formatDate(book.created_at) + '</span>' : '') +
        '</div>';
      }).join('') + '</div>';
    } catch (e) {
      el.innerHTML = '<p class="empty">加载失败</p>';
    }
  }
};
