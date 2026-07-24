(function () {
    'use strict';

    var requestCode = window.location.pathname.split('/').pop();
    var fileListEl = document.getElementById('file-list');
    var uploadResultEl = document.getElementById('upload-result');
    var uploadFormEl = document.getElementById('upload-form');

    document.addEventListener('DOMContentLoaded', function () {
        loadRequest();
        if (uploadFormEl) {
            uploadFormEl.addEventListener('submit', function (event) {
                event.preventDefault();
                uploadFiles();
            });
        }
    });

    function loadRequest() {
        fetch('/api/requests/' + requestCode)
            .then(handleJson)
            .then(function (data) {
                renderRequest(data.request);
                renderFiles(data.request.files || []);
            })
            .catch(function () {
                setText('request-title', 'Request not found');
                setText('request-desc', 'This file request link is invalid or expired.');
            });
    }

    function renderRequest(req) {
        setText('request-title', req.title);
        setText('request-desc', req.description || 'Please upload the requested files.');

        setText('expires-chip', req.expires_at ? ('Expires ' + formatDate(req.expires_at)) : 'No expiry');
        setText('limit-chip', req.max_files ? ('Max ' + req.max_files + ' files') : 'Unlimited files');
    }

    function renderFiles(files) {
        if (!fileListEl) return;
        if (!files.length) {
            fileListEl.innerHTML = '<div class="empty">No files yet.</div>';
            return;
        }

        var html = '';
        files.forEach(function (file) {
            html +=
                '<div class="request-card">' +
                '<h4>' + escapeHtml(file.original_filename) + '</h4>' +
                '<div class="request-meta">' +
                '<span>Size: ' + formatBytes(file.size) + '</span>' +
                '<span>Uploaded: ' + formatDate(file.uploaded_at) + '</span>' +
                '</div>' +
                '<div class="request-actions">' +
                '<a class="primary-link" href="/download/' + requestCode + '/' + file.id + '">Download</a>' +
                '</div>' +
                '</div>';
        });
        fileListEl.innerHTML = html;
    }

    function uploadFiles() {
        if (!uploadFormEl) return;
        var formData = new FormData(uploadFormEl);

        fetch('/api/requests/' + requestCode + '/upload', {
            method: 'POST',
            body: formData
        })
            .then(handleJson)
            .then(function () {
                if (uploadResultEl) uploadResultEl.innerHTML = '<div class="request-card">Upload complete. Thank you.</div>';
                uploadFormEl.reset();
                loadRequest();
            })
            .catch(function (err) {
                var msg = err && err.message ? err.message : 'Upload failed.';
                if (uploadResultEl) uploadResultEl.innerHTML = '<div class="empty">' + escapeHtml(msg) + '</div>';
            });
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

    function setText(id, text) {
        var el = document.getElementById(id);
        if (el) el.textContent = text || '';
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
