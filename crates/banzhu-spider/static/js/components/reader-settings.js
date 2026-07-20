// ─── 阅读器设置组件 ──────────────────────────────────────────────────────────

var ReaderSettings = (function() {
  var KEYS = {
    fontSize: 'reader-font-size',
    lineHeight: 'reader-line-height',
    theme: 'reader-theme',
  };

  var THEMES = [
    { name: '白', cls: 'theme-white', bg: '#ffffff', color: '#1a1a1a' },
    { name: '米黄', cls: 'theme-sepia', bg: '#f5f0e8', color: '#3d3225' },
    { name: '暗绿', cls: 'theme-green', bg: '#1a2e1a', color: '#c8e6c8' },
    { name: '暗黑', cls: 'theme-dark', bg: '#1a1a2e', color: '#d0d0e0' },
  ];

  var LINE_HEIGHTS = [1.5, 1.8, 2.0, 2.5];

  function getFontSize() {
    return parseInt(localStorage.getItem(KEYS.fontSize)) || 18;
  }

  function getLineHeight() {
    return parseFloat(localStorage.getItem(KEYS.lineHeight)) || 2.0;
  }

  function getTheme() {
    return localStorage.getItem(KEYS.theme) || 'theme-white';
  }

  function applyTo(el) {
    if (!el) return;
    el.style.fontSize = getFontSize() + 'px';
    el.style.lineHeight = String(getLineHeight());
    // Remove all theme classes then add current
    THEMES.forEach(function(t) { el.classList.remove(t.cls); });
    el.classList.add(getTheme());
  }

  function renderPanel() {
    var fs = getFontSize();
    var lh = getLineHeight();
    var theme = getTheme();

    var themeButtons = THEMES.map(function(t) {
      var active = t.cls === theme ? ' active' : '';
      return '<button class="rs-theme-btn' + active + '" data-theme="' + t.cls + '" style="background:' + t.bg + ';color:' + t.color + '">' + t.name + '</button>';
    }).join('');

    var lhButtons = LINE_HEIGHTS.map(function(v) {
      var active = v === lh ? ' active' : '';
      return '<button class="rs-lh-btn' + active + '" data-lh="' + v + '">' + v + '</button>';
    }).join('');

    return '<div class="reader-settings" id="readerSettings">' +
      '<div class="rs-header"><span>阅读设置</span><button class="rs-close" id="rsClose">&times;</button></div>' +
      '<div class="rs-section">' +
        '<label>字号 (' + fs + 'px)</label>' +
        '<input type="range" id="rsFontSize" min="14" max="24" value="' + fs + '" class="rs-slider">' +
      '</div>' +
      '<div class="rs-section">' +
        '<label>行距</label>' +
        '<div class="rs-btn-group">' + lhButtons + '</div>' +
      '</div>' +
      '<div class="rs-section">' +
        '<label>主题</label>' +
        '<div class="rs-btn-group">' + themeButtons + '</div>' +
      '</div>' +
    '</div>';
  }

  function bindEvents(contentEl) {
    var panel = document.getElementById('readerSettings');
    if (!panel) return;

    document.getElementById('rsClose').addEventListener('click', function() {
      panel.classList.remove('open');
    });

    var slider = document.getElementById('rsFontSize');
    slider.addEventListener('input', function() {
      var size = parseInt(this.value);
      localStorage.setItem(KEYS.fontSize, String(size));
      panel.querySelector('.rs-section label').textContent = '字号 (' + size + 'px)';
      applyTo(contentEl);
    });

    panel.querySelectorAll('.rs-lh-btn').forEach(function(btn) {
      btn.addEventListener('click', function() {
        panel.querySelectorAll('.rs-lh-btn').forEach(function(b) { b.classList.remove('active'); });
        this.classList.add('active');
        localStorage.setItem(KEYS.lineHeight, this.dataset.lh);
        applyTo(contentEl);
      });
    });

    panel.querySelectorAll('.rs-theme-btn').forEach(function(btn) {
      btn.addEventListener('click', function() {
        panel.querySelectorAll('.rs-theme-btn').forEach(function(b) { b.classList.remove('active'); });
        this.classList.add('active');
        localStorage.setItem(KEYS.theme, this.dataset.theme);
        applyTo(contentEl);
      });
    });
  }

  function toggle(contentEl) {
    var panel = document.getElementById('readerSettings');
    if (panel) {
      panel.classList.toggle('open');
    }
  }

  return {
    applyTo: applyTo,
    renderPanel: renderPanel,
    bindEvents: bindEvents,
    toggle: toggle,
    getFontSize: getFontSize,
    getLineHeight: getLineHeight,
    getTheme: getTheme,
  };
})();
