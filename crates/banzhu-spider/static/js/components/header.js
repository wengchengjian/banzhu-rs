// ─── Header 组件 ─────────────────────────────────────────────────────────────

function renderHeader() {
  const isDark = document.body.classList.contains('dark');
  return `
  <header class="site-header">
    <div class="container header-inner">
      <a href="#/" class="logo">版主网</a>
      <nav class="nav-links">
        <a href="#/">首页</a>
        <a href="#/shelf">书架</a>
        <a href="#/crawler">爬虫</a>
        <a href="#/stats">统计</a>
      </nav>
      <div class="header-actions">
        <form class="search-mini" id="headerSearchForm">
          <input type="text" id="headerSearchInput" placeholder="搜索小说..." autocomplete="off">
          <button type="submit">🔍</button>
        </form>
        <button class="dark-toggle" id="darkModeToggle" title="切换主题">${isDark ? '☀️' : '🌙'}</button>
        <button class="menu-toggle" id="menuToggle">☰</button>
      </div>
    </div>
    <nav class="mobile-nav" id="mobileNav">
      <a href="#/">首页</a>
      <a href="#/shelf">书架</a>
      <a href="#/crawler">爬虫</a>
      <a href="#/stats">统计</a>
    </nav>
  </header>`;
}

function bindHeaderEvents() {
  // 搜索表单
  const form = document.getElementById('headerSearchForm');
  const input = document.getElementById('headerSearchInput');
  if (form && input) {
    form.addEventListener('submit', function(e) {
      e.preventDefault();
      const q = input.value.trim();
      if (q) navigate('#/search?q=' + encodeURIComponent(q));
    });
  }

  // 暗黑模式切换
  const toggle = document.getElementById('darkModeToggle');
  if (toggle) {
    toggle.addEventListener('click', toggleTheme);
  }

  // 移动端菜单
  const menuBtn = document.getElementById('menuToggle');
  const mobileNav = document.getElementById('mobileNav');
  if (menuBtn && mobileNav) {
    menuBtn.addEventListener('click', function() {
      mobileNav.classList.toggle('open');
    });
    mobileNav.querySelectorAll('a').forEach(function(link) {
      link.addEventListener('click', function() {
        mobileNav.classList.remove('open');
      });
    });
  }
}
