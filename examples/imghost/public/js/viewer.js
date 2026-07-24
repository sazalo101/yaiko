/* ============================================================
   ImgHost — Viewer Page Logic
   ============================================================ */

(function () {
    'use strict';

    const errorEl = document.getElementById('viewer-error');
    const contentEl = document.getElementById('viewer-content');

    // Extract image ID from URL path "/i/:id"
    const pathParts = window.location.pathname.split('/');
    const imageId = pathParts[pathParts.length - 1];

    if (!imageId) {
        showError();
        return;
    }

    loadImage(imageId);

    async function loadImage(id) {
        try {
            const res = await fetch('/api/images/' + id);
            if (!res.ok) {
                showError();
                return;
            }

            const img = await res.json();

            // Populate UI
            document.title = img.original_name + ' — ImgHost';
            document.getElementById('viewer-img').src = img.raw_url;
            document.getElementById('viewer-img').alt = img.original_name;
            document.getElementById('viewer-name').textContent = img.original_name;

            // Stats
            document.getElementById('stat-views').textContent = img.view_count.toLocaleString();
            document.getElementById('stat-dimensions').textContent =
                (img.width > 0 && img.height > 0) ? `${img.width}×${img.height}` : 'Auto';
            document.getElementById('stat-filesize').textContent = formatBytes(img.size_bytes);

            const uploadDate = new Date(img.created_at);
            document.getElementById('stat-date').textContent = uploadDate.toLocaleDateString();

            // Links
            document.getElementById('v-direct').value = img.raw_url;
            document.getElementById('v-html').value = `<img src="${img.raw_url}" alt="${img.original_name}">`;
            document.getElementById('v-md').value = `![${img.original_name}](${img.raw_url})`;
            document.getElementById('v-bb').value = `[img]${img.raw_url}[/img]`;

            // Open original button
            document.getElementById('btn-open-raw').href = img.raw_url;

            // Store ID for delete
            window._imgId = id;

            contentEl.style.display = 'grid';
        } catch (e) {
            console.error(e);
            showError();
        }
    }

    function showError() {
        errorEl.style.display = 'block';
        contentEl.style.display = 'none';
    }

    function formatBytes(bytes) {
        if (bytes === 0) return '0 B';
        const k = 1024;
        const sizes = ['B', 'KB', 'MB', 'GB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
    }

    // ---- Copy Helper ----
    window.copyField = function (inputId) {
        const input = document.getElementById(inputId);
        if (!input) return;
        navigator.clipboard.writeText(input.value).then(() => {
            const btn = input.nextElementSibling;
            if (btn) {
                btn.textContent = '✓';
                btn.classList.add('copied');
                setTimeout(() => {
                    btn.textContent = 'Copy';
                    btn.classList.remove('copied');
                }, 1500);
            }
        });
    };

    // ---- Delete Modal ----
    window.showDeleteModal = function () {
        document.getElementById('delete-modal').style.display = 'flex';
        document.getElementById('delete-token-input').value = '';
        document.getElementById('delete-error').style.display = 'none';
    };

    window.hideDeleteModal = function () {
        document.getElementById('delete-modal').style.display = 'none';
    };

    window.confirmDelete = async function () {
        const token = document.getElementById('delete-token-input').value.trim();
        const errEl = document.getElementById('delete-error');

        if (!token) {
            errEl.textContent = 'Please enter your delete token.';
            errEl.style.display = 'block';
            return;
        }

        try {
            const res = await fetch('/api/images/' + window._imgId + '?token=' + encodeURIComponent(token), {
                method: 'DELETE',
            });

            if (res.ok) {
                window.location.href = '/';
            } else {
                const data = await res.json();
                errEl.textContent = data.error || 'Delete failed.';
                errEl.style.display = 'block';
            }
        } catch (e) {
            errEl.textContent = 'Network error.';
            errEl.style.display = 'block';
        }
    };
})();
