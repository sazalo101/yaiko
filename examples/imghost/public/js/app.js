/* ============================================================
   ImgHost — Upload & Gallery App Logic
   ============================================================ */

(function () {
    'use strict';

    const dropzone = document.getElementById('dropzone');
    const fileInput = document.getElementById('file-input');
    const uploadProgress = document.getElementById('upload-progress');
    const progressFill = document.getElementById('progress-fill');
    const progressText = document.getElementById('progress-text');
    const resultsSection = document.getElementById('upload-results');
    const resultsList = document.getElementById('results-list');
    const gallery = document.getElementById('gallery');
    const galleryEmpty = document.getElementById('gallery-empty');

    // ---- Drag & Drop ----
    ['dragenter', 'dragover'].forEach(evt => {
        dropzone.addEventListener(evt, e => {
            e.preventDefault();
            dropzone.classList.add('dragover');
        });
    });

    ['dragleave', 'drop'].forEach(evt => {
        dropzone.addEventListener(evt, e => {
            e.preventDefault();
            dropzone.classList.remove('dragover');
        });
    });

    dropzone.addEventListener('drop', e => {
        const files = e.dataTransfer.files;
        if (files.length) handleUpload(files);
    });

    dropzone.addEventListener('click', () => fileInput.click());
    fileInput.addEventListener('change', () => {
        if (fileInput.files.length) handleUpload(fileInput.files);
    });

    // ---- Upload Handler ----
    async function handleUpload(files) {
        const formData = new FormData();
        for (const f of files) {
            formData.append('file', f, f.name);
        }

        // Show progress
        uploadProgress.style.display = 'block';
        document.querySelector('.dropzone-content').style.display = 'none';
        progressFill.style.width = '10%';
        progressText.textContent = `Uploading ${files.length} file(s)...`;

        try {
            const xhr = new XMLHttpRequest();
            xhr.open('POST', '/api/upload', true);

            xhr.upload.addEventListener('progress', e => {
                if (e.lengthComputable) {
                    const pct = Math.round((e.loaded / e.total) * 100);
                    progressFill.style.width = pct + '%';
                    progressText.textContent = `Uploading... ${pct}%`;
                }
            });

            const response = await new Promise((resolve, reject) => {
                xhr.onload = () => resolve(xhr);
                xhr.onerror = () => reject(new Error('Upload failed'));
                xhr.send(formData);
            });

            progressFill.style.width = '100%';
            progressText.textContent = 'Processing...';

            if (response.status >= 200 && response.status < 300) {
                const data = JSON.parse(response.responseText);
                showResults(data.images);
            } else {
                const err = JSON.parse(response.responseText);
                progressText.textContent = '❌ ' + (err.error || 'Upload failed');
                progressText.style.color = '#ef4444';
            }
        } catch (err) {
            progressText.textContent = '❌ Network error';
            progressText.style.color = '#ef4444';
        }

        // Reset after 2s
        setTimeout(() => {
            uploadProgress.style.display = 'none';
            document.querySelector('.dropzone-content').style.display = 'block';
            progressFill.style.width = '0%';
            progressText.textContent = 'Uploading...';
            progressText.style.color = '';
            fileInput.value = '';
        }, 2500);
    }

    // ---- Show Upload Results ----
    function showResults(images) {
        resultsSection.style.display = 'block';
        resultsList.innerHTML = '';

        images.forEach(img => {
            const template = document.getElementById('result-card-template');
            const card = template.content.cloneNode(true);

            card.querySelector('.result-img').src = img.url;
            card.querySelector('.result-img').alt = img.original_name;
            card.querySelector('.result-name').textContent = img.original_name;
            card.querySelector('.meta-item').textContent = img.mime_type;
            card.querySelector('.meta-size').textContent = formatBytes(img.size_bytes);
            card.querySelector('.direct-link').value = img.url;
            card.querySelector('.viewer-link').value = img.viewer_url;
            card.querySelector('.html-link').value = `<img src="${img.url}" alt="${img.original_name}">`;
            card.querySelector('.md-link').value = `![${img.original_name}](${img.url})`;
            card.querySelector('.delete-token-code').textContent = img.delete_token;

            // Wire up copy buttons
            card.querySelectorAll('.copy-btn').forEach(btn => {
                btn.addEventListener('click', () => {
                    const target = btn.getAttribute('data-target');
                    const input = btn.closest('.result-card').querySelector('.' + target);
                    if (input) {
                        navigator.clipboard.writeText(input.value).then(() => {
                            btn.textContent = '✓';
                            btn.classList.add('copied');
                            setTimeout(() => {
                                btn.textContent = 'Copy';
                                btn.classList.remove('copied');
                            }, 1500);
                        });
                    }
                });
            });

            resultsList.appendChild(card);
        });

        // Scroll to results
        resultsSection.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }

    // ---- Utils ----
    function formatBytes(bytes) {
        if (bytes === 0) return '0 B';
        const k = 1024;
        const sizes = ['B', 'KB', 'MB', 'GB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
    }
})();

