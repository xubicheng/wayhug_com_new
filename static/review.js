// 审阅工作台快捷键与实时预览。没有它，所有按钮仍然可用。
(function () {
  var form = document.getElementById('rv-form');
  if (!form) return;
  var accept = document.getElementById('rv-accept');
  var reject = document.getElementById('rv-reject');
  var reason = document.getElementById('rv-reason');
  var prev = document.getElementById('rv-prev');
  var next = document.getElementById('rv-next');
  var preview = document.getElementById('rv-preview');
  var body = document.getElementById('rv-body');
  var question = document.getElementById('rv-question');
  var csrfEl = form.querySelector('input[name="csrf"]');
  var timer = null;

  function click(el) {
    if (el && el.tagName === 'A' && el.href) {
      window.location.href = el.href;
    } else if (el) {
      el.click();
    }
  }

  function updatePreview() {
    if (!preview || !body || !csrfEl) return;
    var data = new URLSearchParams();
    data.set('csrf', csrfEl.value);
    data.set('body', body.value);
    data.set('question', question ? question.value : '');
    fetch('/admin/review/preview', {
      method: 'POST',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: data.toString(),
      credentials: 'same-origin'
    })
      .then(function (r) {
        if (!r.ok) throw new Error('preview failed');
        return r.text();
      })
      .then(function (html) {
        preview.innerHTML = html;
      })
      .catch(function () {});
  }

  function schedulePreview() {
    window.clearTimeout(timer);
    timer = window.setTimeout(updatePreview, 250);
  }

  if (body) body.addEventListener('input', schedulePreview);
  if (question) question.addEventListener('input', schedulePreview);

  document.addEventListener('keydown', function (event) {
    var el = event.target;
    var tag = el && el.tagName ? el.tagName : '';
    var typing = /^(INPUT|TEXTAREA|SELECT)$/.test(tag);

    if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') {
      if (accept) {
        event.preventDefault();
        accept.click();
      }
      return;
    }
    if (event.ctrlKey || event.metaKey || event.altKey) return;

    if (el === reason && event.key === 'Enter') {
      event.preventDefault();
      if (reject) reject.click();
      return;
    }
    if (typing) return;

    var k = event.key.toLowerCase();
    if (k === 'a') {
      event.preventDefault();
      if (accept) accept.click();
    } else if (k === 'r') {
      event.preventDefault();
      if (reason) reason.focus();
    } else if (k === 'j') {
      event.preventDefault();
      click(next);
    } else if (k === 'k') {
      event.preventDefault();
      click(prev);
    }
  });
})();
