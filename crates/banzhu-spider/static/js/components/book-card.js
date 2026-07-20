// ─── Book Card 组件 ──────────────────────────────────────────────────────────

const BOOK_COLORS = [
  '#2c5282', '#2f855a', '#9b2c2c', '#744210',
  '#553c9a', '#285e61', '#975a16', '#1a365d',
];

function getBookColor(title) {
  let hash = 0;
  for (let i = 0; i < title.length; i++) {
    hash = title.charCodeAt(i) + ((hash << 5) - hash);
  }
  return BOOK_COLORS[Math.abs(hash) % BOOK_COLORS.length];
}

function renderBookCard(book) {
  const firstChar = escapeHtml((book.title || '?').charAt(0));
  const color = getBookColor(book.title || '');
  const wordCount = formatWordCount(book.word_count);
  const category = book.category ? `<span class="book-category">${escapeHtml(book.category)}</span>` : '';

  return `
  <div class="book-card">
    <a href="#/book/${book.id}" class="book-card-link">
      <div class="book-card-top">
        <span class="book-avatar" style="background:${color}">${firstChar}</span>
        <div class="book-card-info">
          <div class="book-card-title">${escapeHtml(book.title)}</div>
          <div class="book-card-meta">
            <span class="book-author">${escapeHtml(book.author || '佚名')}</span>
            ${category}
          </div>
        </div>
      </div>
      <div class="book-card-stats">
        <span>${wordCount}</span>
        ${book.chapter_count ? `<span>${book.chapter_count}章</span>` : ''}
      </div>
      ${book.introduce ? `<div class="book-card-intro">${escapeHtml(book.introduce)}</div>` : ''}
    </a>
  </div>`;
}

function renderBookGrid(books) {
  if (!books || books.length === 0) {
    return '<p class="empty">暂无书籍</p>';
  }
  return `<div class="book-grid">${books.map(renderBookCard).join('')}</div>`;
}
