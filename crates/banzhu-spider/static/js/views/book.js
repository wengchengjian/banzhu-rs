// ─── 书籍详情视图 ────────────────────────────────────────────────────────────

Views.book = async function(app, bookId) {
  app.innerHTML = `${renderHeader()}<main><div class="container"><p class="empty">加载中...</p></div></main>`;
  bindHeaderEvents();

  // 并行获取数据
  let book, chapters, progress, shelfItem;
  try {
    [book, chapters] = await Promise.all([
      API.get('/books/' + bookId),
      API.get('/books/' + bookId + '/chapters'),
    ]);
  } catch (e) {
    app.innerHTML = `${renderHeader()}<main><div class="container"><div class="error-page"><h1>加载失败</h1><p>${escapeHtml(e.message)}</p><a href="#/" class="btn btn-primary">返回首页</a></div></div></main>`;
    bindHeaderEvents();
    return;
  }

  // 获取进度和书架状态（非关键路径）
  try { progress = await API.get('/progress/' + bookId); } catch (e) { progress = null; }
  try {
    const shelf = await API.get('/bookshelf');
    shelfItem = (shelf || []).find(function(s) { return String(s.book_id) === String(bookId); });
  } catch (e) { shelfItem = null; }

  const container = app.querySelector('.container');
  const chapterCount = chapters ? chapters.length : 0;
  const progressOrder = progress ? progress.chapter_order : 0;

  container.innerHTML = `
    <div class="breadcrumb"><a href="#/">首页</a> / ${escapeHtml(book.title)}</div>
    <div class="book-detail">
      <h1>${escapeHtml(book.title)}</h1>
      <div class="book-meta">
        <span>作者：${escapeHtml(book.author || '佚名')}</span>
        <span>分类：${escapeHtml(book.category || '未分类')}</span>
        <span>字数：${formatWordCount(book.word_count)}</span>
        <span>章节：${chapterCount}章</span>
        ${book.created_at ? `<span>收录：${formatDate(book.created_at)}</span>` : ''}
      </div>
      ${book.introduce ? `<div class="book-introduce"><h3>简介</h3><p>${escapeHtml(book.introduce)}</p></div>` : ''}
      <div class="book-actions">
        <button class="btn btn-primary" id="btnRead">${progressOrder > 0 ? '继续阅读' : '开始阅读'}</button>
        <button class="btn btn-outline" id="btnShelf">${shelfItem ? '已在书架 (' + escapeHtml(shelfItem.group_name || '默认') + ')' : '加入书架'}</button>
        <a class="btn btn-outline" href="/api/export/${bookId}?format=txt">导出 TXT</a>
        <a class="btn btn-outline" href="/api/export/${bookId}?format=epub">导出 EPUB</a>
        <button class="btn btn-outline btn-danger" id="btnDelete">删除</button>
      </div>
      ${shelfItem ? `
      <div class="shelf-group-change">
        <label>分组：</label>
        <input type="text" id="shelfGroupInput" value="${escapeHtml(shelfItem.group_name || '')}" placeholder="输入分组名">
        <button class="btn btn-outline btn-sm" id="btnChangeGroup">更新分组</button>
      </div>` : ''}
      <div class="chapter-list">
        <h2>目录 ${progressOrder > 0 ? `<span class="progress-hint">（已读到第${progressOrder}章）</span>` : ''}</h2>
        <div class="chapter-pages" id="chapterPages"></div>
        <div class="pagination" id="chapterPagination"></div>
      </div>
    </div>`;

  // 章节分页渲染
  const perPage = 10;
  const totalPages = Math.ceil(chapterCount / perPage);
  let chapterPage = 1;

  // 如果有进度，定位到对应页
  if (progressOrder > 0) {
    chapterPage = Math.ceil(progressOrder / perPage);
  }

  function renderChapterPage(page) {
    const pagesEl = document.getElementById('chapterPages');
    const pagEl = document.getElementById('chapterPagination');
    const start = (page - 1) * perPage;
    const pageChapters = chapters.slice(start, start + perPage);

    pagesEl.innerHTML = `<ul class="chapter-items">${pageChapters.map(function(ch) {
      const order = ch.chapter_order || (start + pageChapters.indexOf(ch) + 1);
      const isCurrent = order === progressOrder;
      return `<li${isCurrent ? ' class="current-chapter"' : ''}><a href="#/read/${bookId}/${order}"><span class="chapter-index">${order}</span><span class="chapter-title">${escapeHtml(ch.title)}</span></a></li>`;
    }).join('')}</ul>`;

    // 分页控件
    if (totalPages > 1) {
      pagEl.innerHTML = `
        ${page > 1 ? `<a class="page-btn" data-page="${page - 1}">上一页</a>` : ''}
        <span class="page-info">${page} / ${totalPages}</span>
        ${page < totalPages ? `<a class="page-btn" data-page="${page + 1}">下一页</a>` : ''}`;
      pagEl.querySelectorAll('.page-btn').forEach(function(btn) {
        btn.addEventListener('click', function() {
          chapterPage = parseInt(this.dataset.page);
          renderChapterPage(chapterPage);
        });
      });
    } else {
      pagEl.innerHTML = '';
    }
  }

  renderChapterPage(chapterPage);

  // 开始阅读 / 继续阅读
  document.getElementById('btnRead').addEventListener('click', function() {
    const order = progressOrder > 0 ? progressOrder : 1;
    navigate('#/read/' + bookId + '/' + order);
  });

  // 加入书架 / 移出书架
  const btnShelf = document.getElementById('btnShelf');
  btnShelf.addEventListener('click', async function() {
    try {
      if (shelfItem) {
        await API.del('/bookshelf/' + bookId);
        shelfItem = null;
        btnShelf.textContent = '加入书架';
      } else {
        await API.post('/bookshelf', { book_id: parseInt(bookId), group: '默认' });
        shelfItem = { book_id: bookId, group_name: '默认' };
        btnShelf.textContent = '已在书架 (默认)';
      }
    } catch (e) {
      alert('操作失败: ' + e.message);
    }
  });

  // 更新分组
  const btnChangeGroup = document.getElementById('btnChangeGroup');
  if (btnChangeGroup) {
    btnChangeGroup.addEventListener('click', async function() {
      const group = document.getElementById('shelfGroupInput').value.trim();
      try {
        await API.put('/bookshelf/' + bookId, { group: group || '默认' });
        btnShelf.textContent = '已在书架 (' + (group || '默认') + ')';
      } catch (e) {
        alert('更新失败: ' + e.message);
      }
    });
  }

  // 删除书籍
  document.getElementById('btnDelete').addEventListener('click', async function() {
    if (!confirm('确定要删除《' + book.title + '》吗？此操作不可恢复。')) return;
    try {
      await API.del('/books/' + bookId);
      navigate('#/');
    } catch (e) {
      alert('删除失败: ' + e.message);
    }
  });
};
