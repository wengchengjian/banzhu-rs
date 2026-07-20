// ─── SPA 路由器 & 工具函数 ──────────────────────────────────────────────────

const Views = {};

// ─── 路由解析 ────────────────────────────────────────────────────────────────

function parseRoute() {
  const hash = location.hash.slice(1) || '/';
  const [path, queryStr] = hash.split('?');
  const parts = path.split('/').filter(Boolean);
  const params = {};
  if (queryStr) {
    new URLSearchParams(queryStr).forEach((v, k) => { params[k] = v; });
  }
  return { parts, params };
}

function navigate(hash) {
  location.hash = hash;
}

// ─── 渲染入口 ────────────────────────────────────────────────────────────────

async function render() {
  const { parts, params } = parseRoute();
  const app = document.getElementById('app');

  try {
    if (parts.length === 0) {
      await Views.home(app, params);
    } else if (parts[0] === 'book' && parts[1]) {
      await Views.book(app, parts[1]);
    } else if (parts[0] === 'search') {
      await Views.search(app, params);
    } else if (parts[0] === 'read' && parts[1] && parts[2]) {
      if (Views.reader) {
        await Views.reader(app, parts[1], parts[2]);
      } else {
        app.innerHTML = '<div class="container"><p class="empty">阅读器尚未实现</p></div>';
      }
    } else if (parts[0] === 'shelf') {
      if (Views.shelf) {
        await Views.shelf(app);
      } else {
        app.innerHTML = '<div class="container"><p class="empty">书架页面尚未实现</p></div>';
      }
    } else if (parts[0] === 'crawler') {
      if (Views.crawler) {
        await Views.crawler(app);
      } else {
        app.innerHTML = '<div class="container"><p class="empty">爬虫页面尚未实现</p></div>';
      }
    } else if (parts[0] === 'stats') {
      if (Views.stats) {
        await Views.stats(app);
      } else {
        app.innerHTML = '<div class="container"><p class="empty">统计页面尚未实现</p></div>';
      }
    } else {
      app.innerHTML = '<div class="container"><div class="error-page"><h1>404</h1><p>页面不存在</p><a href="#/" class="btn btn-primary">返回首页</a></div></div>';
    }
  } catch (err) {
    app.innerHTML = `<div class="container"><div class="error-page"><h1>出错了</h1><p>${escapeHtml(err.message)}</p><a href="#/" class="btn btn-primary">返回首页</a></div></div>`;
  }

  window.scrollTo(0, 0);
}

// ─── 主题切换 ────────────────────────────────────────────────────────────────

const THEME_KEY = 'banzhu-theme';

function initTheme() {
  const saved = localStorage.getItem(THEME_KEY) || 'light';
  applyTheme(saved);
}

function applyTheme(theme) {
  if (theme === 'dark') {
    document.body.classList.add('dark');
  } else {
    document.body.classList.remove('dark');
  }
  localStorage.setItem(THEME_KEY, theme);
}

function toggleTheme() {
  const isDark = document.body.classList.contains('dark');
  applyTheme(isDark ? 'light' : 'dark');
  // 更新按钮文字
  const btn = document.getElementById('darkModeToggle');
  if (btn) btn.textContent = isDark ? '🌙' : '☀️';
}

// ─── 工具函数 ────────────────────────────────────────────────────────────────

function formatWordCount(n) {
  if (!n && n !== 0) return '0万字';
  if (n >= 10000) return (n / 10000).toFixed(1) + '万字';
  return n + '字';
}

function formatDate(ts) {
  if (!ts) return '';
  return new Date(ts * 1000).toLocaleDateString('zh-CN');
}

function escapeHtml(str) {
  if (!str) return '';
  return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

// ─── 初始化 ──────────────────────────────────────────────────────────────────

window.addEventListener('hashchange', render);
window.addEventListener('DOMContentLoaded', function() {
  initTheme();
  render();
});
