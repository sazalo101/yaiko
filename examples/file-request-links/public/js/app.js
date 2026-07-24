(function () {
    'use strict';

    var listEl = document.getElementById('request-list');
    var resultEl = document.getElementById('create-result');
    var formEl = document.getElementById('create-form');

    document.addEventListener('DOMContentLoaded', function () {
        loadRequests();
        if (formEl) {
            formEl.addEventListener('submit', function (event) {
                event.preventDefault();
                createRequest();
            });
        }
    });

    function loadRequests() {
        fetch('/api/requests')
            .then(handleJson)
            .then(function (data) {
                renderRequests(data.requests || []);
            })
            .catch(function () {
                if (listEl) listEl.innerHTML = '<div class="empty">Failed to load requests.</div>';
            });
    }

    function createRequest() {
        var payload = {
            title: valueOf('input[name="title"]'),
            description: valueOf('textarea[name="description"]'),
            max_files: parseOptionalNumber(valueOf('input[name="max_files"]')),
            expires_in_days: parseOptionalNumber(valueOf('input[name="expires_in_days"]'))
        };

        fetch('/api/requests', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(payload)
        })
            .then(handleJson)
            .then(function (data) {
                var req = data.request;
                var link = window.location.origin + '/r/' + req.code;
                if (resultEl) {
                    resultEl.innerHTML =
                        '<div class="request-card">' +
                        '<strong>Link created:</strong> ' +
                        '<span class="mono">' + escapeHtml(link) + '</span>' +
                        '<div class="request-actions">' +
                        '<button data-link="' + link + '" class="copy-btn">Copy link</button>' +
                        '<a class="primary-link" href="/r/' + req.code + '" target="_blank">Open upload page</a>' +
                        '</div>' +
                        '</div>';
                    var btn = resultEl.querySelector('.copy-btn');
                    if (btn) {
                        btn.addEventListener('click', function () { copyLink(btn.getAttribute('data-link')); });
                    }
                }
                if (formEl) formEl.reset();
                loadRequests();
            })
            .catch(function (err) {
                var msg = err && err.message ? err.message : 'Failed to create request.';
                if (resultEl) resultEl.innerHTML = '<div class="empty">' + escapeHtml(msg) + '</div>';
            });
    }

    function renderRequests(requests) {
        if (!listEl) return;
        if (!requests.length) {
            listEl.innerHTML = '<div class="empty">No requests yet. Create your first link.</div>';
            return;
        }

        var html = '';
        requests.forEach(function (req) {
            var link = window.location.origin + '/r/' + req.code;
            html +=
                '<div class="request-card">' +
                '<h4>' + escapeHtml(req.title) + '</h4>' +
                '<div class="request-meta">' +
                '<span>Files: ' + req.files_count + '</span>' +
                '<span>Total: ' + formatBytes(req.total_size) + '</span>' +
                '<span>Created: ' + formatDate(req.created_at) + '</span>' +
                '</div>' +
                '<p>' + escapeHtml(req.description || 'No description provided.') + '</p>' +
                '<div class="request-actions">' +
                '<button data-link="' + link + '" class="copy-btn">Copy link</button>' +
                '<a class="primary-link" href="/r/' + req.code + '" target="_blank">Open upload page</a>' +
                '</div>' +
                '</div>';
        });
        listEl.innerHTML = html;
        listEl.querySelectorAll('.copy-btn').forEach(function (btn) {
            btn.addEventListener('click', function () { copyLink(btn.getAttribute('data-link')); });
        });
    }

    function copyLink(link) {
        if (navigator.clipboard && window.isSecureContext) {
            navigator.clipboard.writeText(link)
                .then(function () { alert('Link copied'); })
                .catch(function () { legacyCopy(link); });
            return;
        }
        legacyCopy(link);
    }

    function legacyCopy(link) {
        var textarea = document.createElement('textarea');
        textarea.value = link;
        textarea.setAttribute('readonly', '');
        textarea.style.position = 'absolute';
        textarea.style.left = '-9999px';
        document.body.appendChild(textarea);
        textarea.select();
        try {
            var ok = document.execCommand('copy');
            alert(ok ? 'Link copied' : 'Copy failed');
        } catch (err) {
            alert('Copy failed');
        }
        document.body.removeChild(textarea);
    }

    function parseOptionalNumber(value) {
        if (!value) return null;
        var n = parseInt(value, 10);
        return Number.isNaN(n) ? null : n;
    }

    function formatDate(dateStr) {
        return new Date(dateStr).toLocaleDateString();
    }

    function formatBytes(bytes) {
        if (!bytes) return '0 B';
        var units = ['B', 'KB', 'MB', 'GB'];
        var i = 0;
        var size = bytes;
        while (size >= 1024 && i < units.length - 1) {
            size /= 1024;
            i++;
        }
        return size.toFixed(1) + ' ' + units[i];
    }

    function escapeHtml(text) {
        var div = document.createElement('div');
        div.textContent = text || '';
        return div.innerHTML;
    }

    function valueOf(selector) {
        var el = document.querySelector(selector);
        return el ? el.value : '';
    }

    function handleJson(response) {
        if (!response.ok) {
            return response.json().then(function (data) {
                throw new Error((data && data.error) || 'Request failed');
            });
        }
        return response.json();
    }
})();
