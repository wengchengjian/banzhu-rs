// ─── API 客户端 ─────────────────────────────────────────────────────────────

const API = {
  async get(path) {
    const r = await fetch('/api' + path);
    const json = await r.json();
    if (json.code !== 0) throw new Error(json.msg || 'API error');
    return json.data;
  },

  async post(path, body) {
    const r = await fetch('/api' + path, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    const json = await r.json();
    if (json.code !== 0) throw new Error(json.msg || 'API error');
    return json.data;
  },

  async put(path, body) {
    const r = await fetch('/api' + path, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    const json = await r.json();
    if (json.code !== 0) throw new Error(json.msg || 'API error');
    return json.data;
  },

  async del(path) {
    const r = await fetch('/api' + path, { method: 'DELETE' });
    const json = await r.json();
    if (json.code !== 0) throw new Error(json.msg || 'API error');
    return json.data;
  },
};
