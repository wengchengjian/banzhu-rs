// ─── 爬虫管理视图 ────────────────────────────────────────────────────────────

Views.crawler = async function(app) {
  var refreshTimer = null;

  app.innerHTML =
    renderHeader() +
    '<main><div class="container">' +
      '<h1 class="page-title">爬虫管理</h1>' +
      '<div class="crawl-status" id="crawlStatus"><p class="empty">加载中...</p></div>' +
      '<div class="crawl-manual">' +
        '<h2>手动爬取</h2>' +
        '<form class="crawl-form" id="crawlForm">' +
          '<input type="text" id="crawlUrlInput" placeholder="输入书籍URL..." autocomplete="off">' +
          '<button type="submit" class="btn btn-primary">开始爬取</button>' +
        '</form>' +
        '<p id="crawlMsg" class="crawl-msg"></p>' +
      '</div>' +
      '<div class="crawl-config" id="crawlConfig"></div>' +
      '<div class="crawl-logs-section">' +
        '<h2>运行日志</h2>' +
        '<div class="crawl-logs" id="crawlLogs"><p class="empty">加载中...</p></div>' +
      '</div>' +
    '</div></main>';

  bindHeaderEvents();

  // 手动爬取表单
  document.getElementById('crawlForm').addEventListener('submit', async function(e) {
    e.preventDefault();
    var url = document.getElementById('crawlUrlInput').value.trim();
    var msgEl = document.getElementById('crawlMsg');
    if (!url) { msgEl.textContent = '请输入URL'; return; }
    msgEl.textContent = '提交中...';
    try {
      await API.post('/crawl/manual', { url: url });
      msgEl.textContent = '已提交爬取任务';
      document.getElementById('crawlUrlInput').value = '';
      fetchStatus();
    } catch (err) {
      msgEl.textContent = '失败: ' + err.message;
    }
  });

  function renderStatus(status) {
    var el = document.getElementById('crawlStatus');
    if (!status) {
      el.innerHTML = '<p class="empty">无法获取状态</p>';
      return;
    }
    var running = status.running || status.status === 'running';
    var badge = running
      ? '<span class="crawl-badge running">运行中</span>'
      : '<span class="crawl-badge idle">空闲</span>';

    el.innerHTML =
      '<div class="crawl-status-card">' +
        '<div class="crawl-status-row"><span class="crawl-label">状态</span>' + badge + '</div>' +
        '<div class="crawl-status-row"><span class="crawl-label">当前页</span><span>' + (status.current_page || 0) + '</span></div>' +
        '<div class="crawl-status-row"><span class="crawl-label">发现书籍</span><span>' + (status.books_found || 0) + '</span></div>' +
        '<div class="crawl-status-row"><span class="crawl-label">已下载</span><span>' + (status.books_downloaded || 0) + '</span></div>' +
      '</div>';

    // 配置信息
    var configEl = document.getElementById('crawlConfig');
    var schedule = status.schedule || '未设置';
    var pagesLimit = status.pages_limit || '-';
    configEl.innerHTML =
      '<div class="crawl-config-card">' +
        '<span>定时: ' + escapeHtml(String(schedule)) + '</span>' +
        '<span>页数限制: ' + escapeHtml(String(pagesLimit)) + '</span>' +
      '</div>';
  }

  function renderLogs(logs) {
    var el = document.getElementById('crawlLogs');
    if (!logs || logs.length === 0) {
      el.innerHTML = '<p class="empty">暂无日志</p>';
      return;
    }
    el.innerHTML = logs.map(function(log) {
      var level = (log.level || 'INFO').toUpperCase();
      var cls = 'log-level-' + level.toLowerCase();
      var time = log.timestamp ? formatDate(log.timestamp) : '';
      var msg = escapeHtml(log.message || log.msg || '');
      return '<div class="crawl-log-item">' +
        '<span class="log-badge ' + cls + '">' + level + '</span>' +
        (time ? '<span class="log-time">' + time + '</span>' : '') +
        '<span class="log-msg">' + msg + '</span>' +
      '</div>';
    }).join('');
  }

  async function fetchStatus() {
    try {
      var status = await API.get('/crawl/status');
      renderStatus(status);
    } catch (e) {
      renderStatus(null);
    }
  }

  async function fetchLogs() {
    try {
      var logs = await API.get('/crawl/logs?limit=100');
      renderLogs(logs);
    } catch (e) {
      document.getElementById('crawlLogs').innerHTML = '<p class="empty">日志加载失败</p>';
    }
  }

  await fetchStatus();
  await fetchLogs();

  // 自动刷新 3s
  refreshTimer = setInterval(function() {
    fetchStatus();
    fetchLogs();
  }, 3000);

  // 清理：监听路由变化时清除定时器
  var originalHash = location.hash;
  function checkCleanup() {
    if (location.hash !== originalHash) {
      clearInterval(refreshTimer);
      window.removeEventListener('hashchange', checkCleanup);
    }
  }
  window.addEventListener('hashchange', checkCleanup);
};
