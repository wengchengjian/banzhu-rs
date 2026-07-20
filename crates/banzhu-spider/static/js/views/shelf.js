// ─── 书架视图 ────────────────────────────────────────────────────────────────

Views.shelf = async function(app) {
  var groups = [
    { key: 'reading', label: '在读' },
    { key: 'want', label: '想读' },
    { key: 'finished', label: '读完' },
  ];
  var activeGroup = 'reading';

  app.innerHTML =
    renderHeader() +
    '<main><div class="container">' +
      '<h1 class="page-title">我的书架</h1>' +
      '<div class="shelf-tabs" id="shelfTabs">' +
        groups.map(function(g) {
          return '<button class="shelf-tab' + (g.key === activeGroup ? ' active' : '') + '" data-group="' + g.key + '">' + g.label + '</button>';
        }).join('') +
      '</div>' +
      '<div id="shelfContent"><p class="empty">加载中...</p></div>' +
    '</div></main>';

  bindHeaderEvents();

  var tabsEl = document.getElementById('shelfTabs');
  tabsEl.addEventListener('click', function(e) {
    var btn = e.target.closest('.shelf-tab');
    if (!btn) return;
    activeGroup = btn.dataset.group;
    tabsEl.querySelectorAll('.shelf-tab').forEach(function(b) { b.classList.remove('active'); });
    btn.classList.add('active');
    loadShelf();
  });

  async function loadShelf() {
    var contentEl = document.getElementById('shelfContent');
    contentEl.innerHTML = '<p class="empty">加载中...</p>';

    try {
      var items = await API.get('/bookshelf?group=' + activeGroup);
      if (!items || items.length === 0) {
        contentEl.innerHTML = '<div class="empty">书架空空如也，去<a href="#/">首页</a>添加书籍吧</div>';
        return;
      }

      // 获取每本书的信息和进度
      var cards = await Promise.all(items.map(async function(item) {
        var book = null;
        var progress = null;
        try { book = await API.get('/books/' + item.book_id); } catch (e) { /* skip */ }
        try { progress = await API.get('/progress/' + item.book_id); } catch (e) { /* skip */ }
        return { item: item, book: book, progress: progress };
      }));

      contentEl.innerHTML = '<div class="shelf-list">' + cards.map(function(c) {
        return renderShelfCard(c.item, c.book, c.progress);
      }).join('') + '</div>';

      bindShelfActions();
    } catch (e) {
      contentEl.innerHTML = '<p class="empty">加载失败: ' + escapeHtml(e.message) + '</p>';
    }
  }

  function renderShelfCard(item, book, progress) {
    var title = book ? escapeHtml(book.title) : '书籍 #' + item.book_id;
    var author = book ? escapeHtml(book.author || '佚名') : '';
    var totalChapters = book && book.chapter_count ? book.chapter_count : 0;
    var progressOrder = progress ? progress.chapter_order : 0;
    var percent = totalChapters > 0 ? Math.round((progressOrder / totalChapters) * 100) : 0;

    var progressBar = '';
    if (progressOrder > 0 && totalChapters > 0) {
      progressBar =
        '<div class="shelf-progress-bar">' +
          '<div class="shelf-progress-fill" style="width:' + percent + '%"></div>' +
        '</div>' +
        '<span class="shelf-progress-text">' + progressOrder + '/' + totalChapters + '章 (' + percent + '%)</span>';
    }

    return '<div class="shelf-card" data-book-id="' + item.book_id + '">' +
      '<div class="shelf-card-info">' +
        '<div class="shelf-card-title">' + title + '</div>' +
        (author ? '<div class="shelf-card-author">' + author + '</div>' : '') +
        progressBar +
      '</div>' +
      '<div class="shelf-card-actions">' +
        (progressOrder > 0 ? '<a class="btn btn-primary btn-sm" href="#/read/' + item.book_id + '/' + progressOrder + '">继续阅读</a>' : '<a class="btn btn-outline btn-sm" href="#/book/' + item.book_id + '">查看</a>') +
        '<div class="shelf-dropdown">' +
          '<button class="btn btn-outline btn-sm shelf-menu-btn">&#8943;</button>' +
          '<div class="shelf-dropdown-menu">' +
            '<button class="shelf-menu-item" data-action="move" data-group="reading">移到在读</button>' +
            '<button class="shelf-menu-item" data-action="move" data-group="want">移到想读</button>' +
            '<button class="shelf-menu-item" data-action="move" data-group="finished">移到读完</button>' +
            '<button class="shelf-menu-item danger" data-action="remove">移出书架</button>' +
          '</div>' +
        '</div>' +
      '</div>' +
    '</div>';
  }

  function bindShelfActions() {
    document.querySelectorAll('.shelf-menu-btn').forEach(function(btn) {
      btn.addEventListener('click', function(e) {
        e.stopPropagation();
        var menu = this.nextElementSibling;
        // 关闭其他菜单
        document.querySelectorAll('.shelf-dropdown-menu.open').forEach(function(m) {
          if (m !== menu) m.classList.remove('open');
        });
        menu.classList.toggle('open');
      });
    });

    document.querySelectorAll('.shelf-menu-item').forEach(function(btn) {
      btn.addEventListener('click', async function(e) {
        e.stopPropagation();
        var card = this.closest('.shelf-card');
        var bookId = card.dataset.bookId;
        var action = this.dataset.action;

        try {
          if (action === 'move') {
            await API.put('/bookshelf/' + bookId, { group: this.dataset.group });
          } else if (action === 'remove') {
            await API.del('/bookshelf/' + bookId);
          }
          loadShelf();
        } catch (err) {
          alert('操作失败: ' + err.message);
        }
      });
    });

    // 点击空白处关闭菜单
    document.addEventListener('click', function closeMenus() {
      document.querySelectorAll('.shelf-dropdown-menu.open').forEach(function(m) {
        m.classList.remove('open');
      });
    });
  }

  await loadShelf();
};
