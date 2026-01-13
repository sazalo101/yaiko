/**
 * Yaiko Notes - JavaScript
 */
(function ($) {
    'use strict';

    var currentNoteId = null;

    // Load notes on page load
    $(document).ready(function () {
        loadNotes();

        // New note button
        $('#new-note-btn').on('click', function () {
            createNewNote();
        });

        // Save button
        $('#save-btn').on('click', function () {
            saveNote();
        });

        // Delete button
        $('#delete-btn').on('click', function () {
            deleteNote();
        });

        // Search
        $('#search').on('input', function () {
            var query = $(this).val().toLowerCase();
            filterNotes(query);
        });
    });

    function loadNotes() {
        $.ajax({
            url: '/api/notes',
            method: 'GET',
            dataType: 'json',
            success: function (data) {
                renderNoteList(data.notes);
            },
            error: function (xhr, status, error) {
                console.error('Failed to load notes:', error);
            }
        });
    }

    function renderNoteList(notes) {
        var $list = $('#note-list');
        $list.empty();

        if (notes.length === 0) {
            $list.append('<li class="note-item"><p>No notes yet</p></li>');
            return;
        }

        notes.forEach(function (note) {
            var $item = $('<li>')
                .addClass('note-item')
                .attr('data-id', note.id)
                .html('<h3>' + escapeHtml(note.title) + '</h3><p>' + escapeHtml(note.content.substring(0, 50)) + '</p>');

            if (note.id === currentNoteId) {
                $item.addClass('active');
            }

            $item.on('click', function () {
                selectNote(note.id);
            });

            $list.append($item);
        });
    }

    function selectNote(id) {
        currentNoteId = id;

        // Update active state in list
        $('.note-item').removeClass('active');
        $('.note-item[data-id="' + id + '"]').addClass('active');

        // Load note content
        $.ajax({
            url: '/api/notes/' + id,
            method: 'GET',
            dataType: 'json',
            success: function (data) {
                showEditor(data.note);
            },
            error: function (xhr, status, error) {
                console.error('Failed to load note:', error);
            }
        });
    }

    function showEditor(note) {
        $('#editor-empty').hide();
        $('#editor-content').show();

        $('#note-id').val(note.id);
        $('#note-title').val(note.title);
        $('#note-content').val(note.content);
    }

    function hideEditor() {
        $('#editor-empty').show();
        $('#editor-content').hide();
        $('#note-id').val('');
        $('#note-title').val('');
        $('#note-content').val('');
        currentNoteId = null;
    }

    function createNewNote() {
        var newNote = {
            title: 'Untitled Note',
            content: ''
        };

        $.ajax({
            url: '/api/notes',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify(newNote),
            dataType: 'json',
            success: function (data) {
                loadNotes();
                selectNote(data.note.id);
            },
            error: function (xhr, status, error) {
                console.error('Failed to create note:', error);
            }
        });
    }

    function saveNote() {
        var id = $('#note-id').val();
        if (!id) return;

        var updatedNote = {
            title: $('#note-title').val(),
            content: $('#note-content').val()
        };

        $.ajax({
            url: '/api/notes/' + id,
            method: 'PUT',
            contentType: 'application/json',
            data: JSON.stringify(updatedNote),
            dataType: 'json',
            success: function (data) {
                loadNotes();
            },
            error: function (xhr, status, error) {
                console.error('Failed to save note:', error);
            }
        });
    }

    function deleteNote() {
        var id = $('#note-id').val();
        if (!id) return;

        if (!confirm('Are you sure you want to delete this note?')) {
            return;
        }

        $.ajax({
            url: '/api/notes/' + id,
            method: 'DELETE',
            dataType: 'json',
            success: function (data) {
                hideEditor();
                loadNotes();
            },
            error: function (xhr, status, error) {
                console.error('Failed to delete note:', error);
            }
        });
    }

    function filterNotes(query) {
        $('.note-item').each(function () {
            var $item = $(this);
            var title = $item.find('h3').text().toLowerCase();
            var content = $item.find('p').text().toLowerCase();

            if (title.indexOf(query) > -1 || content.indexOf(query) > -1) {
                $item.show();
            } else {
                $item.hide();
            }
        });
    }

    function escapeHtml(text) {
        var div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

})(jQuery);
