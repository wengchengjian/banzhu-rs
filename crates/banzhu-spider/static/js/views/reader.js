// ─── 阅读器视图 ──────────────────────────────────────────────────────────────

Views.reader = async function(app, bookId, order) {
  order = parseInt(order) || 1;
  var isMobile = window.innerWidth < 768;
  var cleanupFns = [];

  function cleanup() {
    cleanupFns.forEach(function(fn) { fn(); });
    cleanupFns = [];
  }

  // 进度恢复：如果 URL 没有指定章节则重定向
  if (!order || order < 1) {
    try {
      var prog = await API.get('/progress/' + bookId);
      if (prog && prog.chapter_order > 0) {
        navigate('#/read/' + bookId + '/' + prog.chapter_order);
        return;
      }
    } catch (e) { /* ignore */ }
    order = 1;
  }

  // 获取章节内容
  var chapter;
  try {
    chapter = await API.get('/books/' + bookId + '/chapters/' + order);
  } catch (e) {
    app.innerHTML = renderHeader() + '<main><div class="container"><div class="error-page"><h1>加载失败</h1><p>' + escapeHtml(e.message) + '</p><a href="#/book/' + bookId + '" class="btn btn-primary">返回书籍</a></div></div></main>';
    bindHeaderEvents();
    return;
  }

  // 保存进度
  try {
    await API.put('/progress/' + bookId, { chapter_order: order, page_index: 0 });
  } catch (e) { /* ignore */ }

  var content = chapter.content || '';
  var paragraphs = content.split(/\n\n+/).filter(function(p) { return p.trim(); });

  if (isMobile) {
    renderMobile(app, bookId, order, chapter, paragraphs, cleanup, cleanupFns);
  } else {
    renderDesktop(app, bookId, order, chapter, paragraphs, cleanup, cleanupFns);
  }
};

// ─── 桌面模式 ────────────────────────────────────────────────────────────────

function renderDesktop(app, bookId, order, chapter, paragraphs, cleanup, cleanupFns) {
  var contentHtml = paragraphs.map(function(p) {
    return '<p>' + escapeHtml(p) + '</p>';
  }).join('');

  app.innerHTML =
    '<div class="reader-wrapper">' +
      '<div class="reader-top-bar" id="readerTopBar">' +
        '<a href="#/book/' + bookId + '" class="reader-back">&larr; 返回</a>' +
        '<span class="reader-title-small">' + escapeHtml(chapter.title) + '</span>' +
        '<button class="reader-gear" id="readerGear">&#9881;</button>' +
      '</div>' +
      '<div class="reader-body">' +
        '<aside class="reader-sidebar" id="readerSidebar">' +
          '<div class="rs-sidebar-header"><span>目录</span><button id="sidebarClose">&times;</button></div>' +
          '<ul class="reader-sidebar-list" id="sidebarList"><li class="empty">加载中...</li></ul>' +
        '</aside>' +
        '<div class="reader-sidebar-overlay" id="sidebarOverlay"></div>' +
        '<main class="reader-main">' +
          '<div class="reader-content" id="readerContent">' +
            '<h1 class="reader-chapter-title">' + escapeHtml(chapter.title) + '</h1>' +
            contentHtml +
          '</div>' +
          '<div class="reader-toolbar" id="readerToolbar">' +
            '<button class="btn btn-outline" id="btnPrev"' + (order <= 1 ? ' disabled' : '') + '>&larr; 上一章</button>' +
            '<button class="btn btn-outline" id="btnCatalog">目录</button>' +
            '<button class="btn btn-outline" id="btnNext">下一章 &rarr;</button>' +
          '</div>' +
        '</main>' +
      '</div>' +
      ReaderSettings.renderPanel() +
    '</div>';

  var contentEl = document.getElementById('readerContent');
  ReaderSettings.applyTo(contentEl);
  ReaderSettings.bindEvents(contentEl);

  // 设置按钮
  document.getElementById('readerGear').addEventListener('click', function() {
    ReaderSettings.toggle(contentEl);
  });

  // 上一章 / 下一章
  document.getElementById('btnPrev').addEventListener('click', function() {
    if (order > 1) navigate('#/read/' + bookId + '/' + (order - 1));
  });
  document.getElementById('btnNext').addEventListener('click', function() {
    navigate('#/read/' + bookId + '/' + (order + 1));
  });

  // 目录侧边栏
  var sidebar = document.getElementById('readerSidebar');
  var overlay = document.getElementById('sidebarOverlay');

  function openSidebar() {
    sidebar.classList.add('open');
    overlay.classList.add('open');
    loadSidebarChapters();
  }
  function closeSidebar() {
    sidebar.classList.remove('open');
    overlay.classList.remove('open');
  }

  document.getElementById('btnCatalog').addEventListener('click', openSidebar);
  document.getElementById('sidebarClose').addEventListener('click', closeSidebar);
  overlay.addEventListener('click', closeSidebar);

  var sidebarLoaded = false;
  function loadSidebarChapters() {
    if (sidebarLoaded) return;
    sidebarLoaded = true;
    API.get('/books/' + bookId + '/chapters').then(function(chapters) {
      var listEl = document.getElementById('sidebarList');
      listEl.innerHTML = chapters.map(function(ch) {
        var chOrder = ch.chapter_order || 0;
        var cls = chOrder === order ? ' class="current"' : '';
        return '<li' + cls + '><a href="#/read/' + bookId + '/' + chOrder + '">' +
          '<span class="ch-num">' + chOrder + '</span>' +
          '<span class="ch-title">' + escapeHtml(ch.title) + '</span></a></li>';
      }).join('');
      // 滚动到当前章节
      var currentEl = listEl.querySelector('li.current');
      if (currentEl) currentEl.scrollIntoView({ block: 'center' });
    }).catch(function() {
      document.getElementById('sidebarList').innerHTML = '<li class="empty">加载失败</li>';
    });
  }
}

// ─── 移动模式 ────────────────────────────────────────────────────────────────

function renderMobile(app, bookId, order, chapter, paragraphs, cleanup, cleanupFns) {
  // 分页：每页约800字符
  var pages = paginateContent(paragraphs, 800);
  var currentPage = 0;
  var totalPages = pages.length;
  var topBarVisible = true;

  app.innerHTML =
    '<div class="reader-wrapper reader-mobile">' +
      '<div class="reader-top-bar" id="readerTopBar">' +
        '<a href="#/book/' + bookId + '" class="reader-back">&larr;</a>' +
        '<span class="reader-title-small">' + escapeHtml(chapter.title) + '</span>' +
        '<button class="reader-gear" id="readerGear">&#9881;</button>' +
      '</div>' +
      '<div class="reader-mobile-content" id="readerMobileContent">' +
        '<div class="reader-content" id="readerContent"></div>' +
      '</div>' +
      '<div class="reader-page-indicator" id="pageIndicator"></div>' +
      '<div class="reader-mobile-nav" id="mobileNav">' +
        '<button class="btn btn-outline btn-sm" id="btnPrevPage" disabled>&larr; 上一页</button>' +
        '<button class="btn btn-outline btn-sm" id="btnNextChapter" style="display:none">下一章 &rarr;</button>' +
        '<button class="btn btn-outline btn-sm" id="btnNextPage">下一页 &rarr;</button>' +
      '</div>' +
      ReaderSettings.renderPanel() +
    '</div>';

  var contentEl = document.getElementById('readerContent');
  var indicatorEl = document.getElementById('pageIndicator');
  ReaderSettings.applyTo(contentEl);
  ReaderSettings.bindEvents(contentEl);

  function renderPage() {
    var pageContent = pages[currentPage];
    contentEl.innerHTML = pageContent.map(function(p) {
      return '<p>' + escapeHtml(p) + '</p>';
    }).join('');
    indicatorEl.textContent = '第 ' + (currentPage + 1) + '/' + totalPages + ' 页';

    document.getElementById('btnPrevPage').disabled = currentPage === 0;
    var isLast = currentPage === totalPages - 1;
    document.getElementById('btnNextPage').style.display = isLast ? 'none' : '';
    document.getElementById('btnNextChapter').style.display = isLast ? '' : 'none';

    // 保存进度
    API.put('/progress/' + bookId, { chapter_order: order, page_index: currentPage }).catch(function() {});
  }

  renderPage();

  // 翻页按钮
  document.getElementById('btnPrevPage').addEventListener('click', function() {
    if (currentPage > 0) { currentPage--; renderPage(); }
  });
  document.getElementById('btnNextPage').addEventListener('click', function() {
    if (currentPage < totalPages - 1) { currentPage++; renderPage(); }
  });
  document.getElementById('btnNextChapter').addEventListener('click', function() {
    navigate('#/read/' + bookId + '/' + (order + 1));
  });

  // 设置
  document.getElementById('readerGear').addEventListener('click', function() {
    ReaderSettings.toggle(contentEl);
  });

  // 触摸滑动
  var touchStartX = 0;
  var touchStartY = 0;
  var contentArea = document.getElementById('readerMobileContent');

  function onTouchStart(e) {
    touchStartX = e.touches[0].clientX;
    touchStartY = e.touches[0].clientY;
  }

  function onTouchEnd(e) {
    var deltaX = e.changedTouches[0].clientX - touchStartX;
    var deltaY = e.changedTouches[0].clientY - touchStartY;
    if (Math.abs(deltaX) > 50 && Math.abs(deltaX) > Math.abs(deltaY)) {
      if (deltaX < 0 && currentPage < totalPages - 1) {
        currentPage++; renderPage();
      } else if (deltaX > 0 && currentPage > 0) {
        currentPage--; renderPage();
      }
    }
  }

  contentArea.addEventListener('touchstart', onTouchStart, { passive: true });
  contentArea.addEventListener('touchend', onTouchEnd, { passive: true });
  cleanupFns.push(function() {
    contentArea.removeEventListener('touchstart', onTouchStart);
    contentArea.removeEventListener('touchend', onTouchEnd);
  });

  // 点击区域：左30%上一页，右30%下一页，中间切换顶栏
  contentArea.addEventListener('click', function(e) {
    var rect = contentArea.getBoundingClientRect();
    var x = (e.clientX - rect.left) / rect.width;
    if (x < 0.3) {
      if (currentPage > 0) { currentPage--; renderPage(); }
    } else if (x > 0.7) {
      if (currentPage < totalPages - 1) { currentPage++; renderPage(); }
    } else {
      topBarVisible = !topBarVisible;
      document.getElementById('readerTopBar').style.display = topBarVisible ? '' : 'none';
      document.getElementById('mobileNav').style.display = topBarVisible ? '' : 'none';
      indicatorEl.style.display = topBarVisible ? '' : 'none';
    }
  });
}

// ─── 分页工具 ────────────────────────────────────────────────────────────────

function paginateContent(paragraphs, charsPerPage) {
  var pages = [];
  var currentPage = [];
  var currentLen = 0;

  for (var i = 0; i < paragraphs.length; i++) {
    var p = paragraphs[i];
    if (currentLen + p.length > charsPerPage && currentPage.length > 0) {
      pages.push(currentPage);
      currentPage = [];
      currentLen = 0;
    }
    currentPage.push(p);
    currentLen += p.length;
  }
  if (currentPage.length > 0) pages.push(currentPage);
  if (pages.length === 0) pages.push(['（本章暂无内容）']);
  return pages;
}
