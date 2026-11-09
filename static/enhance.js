// 可选增强：防重复提交。没有它，页面依然完整可用。
(function () {
  document.addEventListener('submit', function (event) {
    var form = event.target;
    if (!(form instanceof HTMLFormElement) || form.dataset.submitting === '1') return;
    form.dataset.submitting = '1';
    var btn = form.querySelector('button[type="submit"]');
    if (btn) {
      btn.setAttribute('aria-busy', 'true');
      btn.style.opacity = '0.6';
      btn.style.pointerEvents = 'none';
    }
    window.setTimeout(function () {
      form.dataset.submitting = '';
      if (btn) {
        btn.removeAttribute('aria-busy');
        btn.style.opacity = '';
        btn.style.pointerEvents = '';
      }
    }, 8000);
  });

  // 成功页清除草稿。
  if (document.getElementById('rn-success')) {
    try { window.localStorage.removeItem('wh_reader_draft_v1'); } catch (e) {}
  }

  // 读者笔记：草稿、星期、键盘遮挡。无 JS 时表单照常可提交。
  var form = document.getElementById('rn-form');
  if (!form) return;
  var KEY = 'wh_reader_draft_v1';
  var fields = [
    'occurred_at', 'weekday', 'place', 'weather', 'temp_c', 'book_title',
    'isbn', 'title', 'body', 'question', 'color', 'flag', 'links'
  ];
  var banner = document.getElementById('rn-draft');
  var saveTimer = null;

  function readDraft() {
    try {
      return JSON.parse(window.localStorage.getItem(KEY) || 'null');
    } catch (e) {
      return null;
    }
  }

  function saveDraft() {
    var data = {};
    var has = false;
    fields.forEach(function (name) {
      var el = form.elements[name];
      if (!el) return;
      var value = '';
      if (el.length && !el.tagName) {
        for (var i = 0; i < el.length; i++) {
          if (el[i].checked) value = el[i].value;
        }
      } else if (el.type === 'radio') {
        if (el.checked) value = el.value;
      } else {
        value = el.value;
      }
      data[name] = value;
      if (value && name !== 'occurred_at' && name !== 'weekday') has = true;
    });
    try {
      if (has) window.localStorage.setItem(KEY, JSON.stringify(data));
      else window.localStorage.removeItem(KEY);
    } catch (e) {}
  }

  function setField(name, value) {
    var el = form.elements[name];
    if (!el) return;
    if (el.length && !el.tagName) {
      for (var i = 0; i < el.length; i++) {
        if (el[i].value === value) el[i].checked = true;
      }
      return;
    }
    if (el.type === 'radio') {
      el.checked = el.value === value;
      return;
    }
    el.value = value;
  }

  var draft = readDraft();
  if (draft) {
    var hasContent = false;
    fields.forEach(function (name) {
      if (draft[name] && ['occurred_at', 'weekday'].indexOf(name) < 0) hasContent = true;
    });
    if (hasContent) {
      fields.forEach(function (name) {
        if (typeof draft[name] === 'string') setField(name, draft[name]);
      });
      if (banner) banner.classList.add('show');
    }
  }

  var clearBtn = document.getElementById('rn-draft-clear');
  if (clearBtn) {
    clearBtn.addEventListener('click', function () {
      try { window.localStorage.removeItem(KEY); } catch (e) {}
      if (banner) banner.classList.remove('show');
      form.reset();
    });
  }

  form.addEventListener('input', function () {
    window.clearTimeout(saveTimer);
    saveTimer = window.setTimeout(saveDraft, 400);
  });
  form.addEventListener('change', function () {
    window.clearTimeout(saveTimer);
    saveTimer = window.setTimeout(saveDraft, 200);
  });

  // 选日期后自动算星期（周一=0），用户仍可手动改。
  var dateEl = form.elements['occurred_at'];
  var weekEl = form.elements['weekday'];
  if (dateEl && weekEl) {
    dateEl.addEventListener('change', function () {
      var m = /^(\d{4})-(\d{2})-(\d{2})/.exec(dateEl.value);
      if (!m) return;
      var d = new Date(Date.UTC(+m[1], +m[2] - 1, +m[3]));
      weekEl.value = String((d.getUTCDay() + 6) % 7);
      saveDraft();
    });
  }

  // 键盘弹出时把当前输入滚到可视区中部。
  form.addEventListener('focusin', function (event) {
    var el = event.target;
    if (!el || !/^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName)) return;
    window.setTimeout(function () {
      if (window.innerWidth < 720) {
        el.scrollIntoView({ block: 'center', behavior: 'smooth' });
      }
    }, 250);
  });

})();
