/**
 * Yaiko Chat - Chat JS (Updated for new UI)
 */
(function ($) {
    'use strict';

    var token = localStorage.getItem('token');
    var currentConversationId = null;
    var isLoading = false;

    $(document).ready(function () {
        // Check authentication
        if (!token) {
            window.location.href = '/';
            return;
        }

        // Load user info
        var user = JSON.parse(localStorage.getItem('user') || '{}');
        var email = user.email || 'User';
        $('#user-email').text(email);
        $('#user-avatar').text(email.charAt(0).toUpperCase());

        // Load conversations
        loadConversations();

        // New chat button
        $('#new-chat-btn').on('click', function () {
            startNewChat();
        });

        // Logout button
        $('#logout-btn').on('click', function () {
            $.ajax({
                url: '/api/logout',
                method: 'POST',
                headers: { 'Authorization': 'Bearer ' + token },
                complete: function () {
                    localStorage.removeItem('token');
                    localStorage.removeItem('user');
                    window.location.href = '/';
                }
            });
        });

        // Chat form
        $('#chat-form').on('submit', function (e) {
            e.preventDefault();
            if (isLoading) return;

            var message = $('#chat-input').val().trim();
            if (!message) return;

            sendMessage(message);
        });

        // Enable/disable send button based on input
        $('#chat-input').on('input', function () {
            var hasContent = $(this).val().trim().length > 0;
            $('#send-btn').prop('disabled', !hasContent);

            // Auto-resize
            this.style.height = 'auto';
            this.style.height = Math.min(this.scrollHeight, 200) + 'px';
        });

        // Enter to send (Shift+Enter for new line)
        $('#chat-input').on('keydown', function (e) {
            if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                if (!$('#send-btn').prop('disabled')) {
                    $('#chat-form').submit();
                }
            }
        });

        // Suggestion buttons
        $(document).on('click', '.suggestion-btn', function () {
            var prompt = $(this).data('prompt');
            if (prompt) {
                sendMessage(prompt);
            }
        });
    });

    function startNewChat() {
        currentConversationId = null;
        $('#chat-messages').html(getWelcomeScreen());
        $('.conversation-item').removeClass('active');
        $('#chat-input').val('').focus();
        $('#send-btn').prop('disabled', true);
    }

    function getWelcomeScreen() {
        return '<div class="welcome-screen" id="welcome-screen">' +
            '<div class="welcome-icon">' +
            '<svg width="40" height="40" viewBox="0 0 48 48" fill="none">' +
            '<circle cx="24" cy="24" r="18" stroke="currentColor" stroke-width="2"/>' +
            '<circle cx="24" cy="24" r="7" fill="currentColor"/>' +
            '</svg>' +
            '</div>' +
            '<h1>How can I help you today?</h1>' +
            '<p>I\'m an AI assistant ready to help with questions, creative tasks, and more.</p>' +
            '<div class="suggestions">' +
            '<button class="suggestion-btn" data-prompt="Explain how AI works in simple terms">' +
            '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4M12 8h.01"/></svg>' +
            'Explain how AI works' +
            '</button>' +
            '<button class="suggestion-btn" data-prompt="Help me write a professional email">' +
            '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z"/><path d="M22 6l-10 7L2 6"/></svg>' +
            'Help me write an email' +
            '</button>' +
            '</div>' +
            '</div>';
    }

    function loadConversations() {
        $.ajax({
            url: '/api/conversations',
            method: 'GET',
            headers: { 'Authorization': 'Bearer ' + token },
            success: function (data) {
                renderConversationList(data.conversations);
            }
        });
    }

    function renderConversationList(conversations) {
        var $list = $('#conversation-list');
        $list.empty();

        if (!conversations || conversations.length === 0) {
            return;
        }

        conversations.forEach(function (conv) {
            var $item = $('<li>')
                .addClass('conversation-item')
                .attr('data-id', conv.id)
                .text(conv.title)
                .on('click', function () {
                    loadConversation(conv.id);
                });

            if (conv.id === currentConversationId) {
                $item.addClass('active');
            }

            $list.append($item);
        });
    }

    function loadConversation(id) {
        currentConversationId = id;

        $('.conversation-item').removeClass('active');
        $('.conversation-item[data-id="' + id + '"]').addClass('active');

        $.ajax({
            url: '/api/conversations/' + id,
            method: 'GET',
            headers: { 'Authorization': 'Bearer ' + token },
            success: function (data) {
                renderMessages(data.conversation.messages);
            }
        });
    }

    function renderMessages(messages) {
        var $container = $('#chat-messages');
        $container.empty();

        messages.forEach(function (msg) {
            $container.append(createMessageElement(msg.role, msg.content));
        });

        scrollToBottom();
    }

    // Clean markdown symbols and emojis from AI response
    function cleanAIResponse(text) {
        if (!text) return '';

        // Remove markdown headers (###, ##, #)
        text = text.replace(/^#{1,6}\s*/gm, '');

        // Remove bold/italic markers (**text**, *text*, __text__, _text_)
        text = text.replace(/\*\*([^*]+)\*\*/g, '$1');
        text = text.replace(/\*([^*]+)\*/g, '$1');
        text = text.replace(/__([^_]+)__/g, '$1');
        text = text.replace(/_([^_]+)_/g, '$1');

        // Remove code blocks (```code```)
        text = text.replace(/```[\s\S]*?```/g, function (match) {
            return match.replace(/```\w*\n?/g, '').replace(/```/g, '');
        });

        // Remove inline code (`code`)
        text = text.replace(/`([^`]+)`/g, '$1');

        // Remove bullet points
        text = text.replace(/^[\-\*]\s+/gm, '• ');

        // Remove emojis
        text = text.replace(/[\u{1F600}-\u{1F64F}]/gu, ''); // Emoticons
        text = text.replace(/[\u{1F300}-\u{1F5FF}]/gu, ''); // Misc Symbols
        text = text.replace(/[\u{1F680}-\u{1F6FF}]/gu, ''); // Transport
        text = text.replace(/[\u{1F700}-\u{1F77F}]/gu, ''); // Alchemical
        text = text.replace(/[\u{1F780}-\u{1F7FF}]/gu, ''); // Geometric
        text = text.replace(/[\u{1F800}-\u{1F8FF}]/gu, ''); // Supplemental
        text = text.replace(/[\u{1F900}-\u{1F9FF}]/gu, ''); // Supplemental
        text = text.replace(/[\u{1FA00}-\u{1FA6F}]/gu, ''); // Chess
        text = text.replace(/[\u{1FA70}-\u{1FAFF}]/gu, ''); // Symbols
        text = text.replace(/[\u{2600}-\u{26FF}]/gu, '');   // Misc symbols
        text = text.replace(/[\u{2700}-\u{27BF}]/gu, '');   // Dingbats

        // Clean up extra whitespace
        text = text.replace(/\n{3,}/g, '\n\n');
        text = text.trim();

        return text;
    }

    function createMessageElement(role, content) {
        var isUser = role === 'user';
        var avatarLetter = isUser ? (localStorage.getItem('user') ? JSON.parse(localStorage.getItem('user')).email.charAt(0).toUpperCase() : 'U') : 'Y';

        // Clean AI responses
        var displayContent = isUser ? content : cleanAIResponse(content);

        return '<div class="message ' + role + '">' +
            '<div class="message-avatar">' + avatarLetter + '</div>' +
            '<div class="message-body">' +
            '<div class="message-role">' + (isUser ? 'You' : 'Yaiko') + '</div>' +
            '<div class="message-content">' + escapeHtml(displayContent) + '</div>' +
            '</div>' +
            '</div>';
    }

    function appendMessage(role, content) {
        var $container = $('#chat-messages');

        // Remove welcome screen
        $container.find('.welcome-screen').remove();

        $container.append(createMessageElement(role, content));
        scrollToBottom();
    }

    function showLoadingMessage() {
        var $container = $('#chat-messages');
        var loadingHtml = '<div class="message assistant loading-message">' +
            '<div class="message-avatar">Y</div>' +
            '<div class="message-body">' +
            '<div class="message-role">Yaiko</div>' +
            '<div class="message-loading"><span></span><span></span><span></span></div>' +
            '</div>' +
            '</div>';
        $container.append(loadingHtml);
        scrollToBottom();
    }

    function removeLoadingMessage() {
        $('.loading-message').remove();
    }

    function sendMessage(message) {
        isLoading = true;
        $('#send-btn').prop('disabled', true);

        // Show user message immediately
        appendMessage('user', message);
        $('#chat-input').val('').css('height', 'auto');

        // Show loading
        showLoadingMessage();

        $.ajax({
            url: '/api/chat',
            method: 'POST',
            headers: { 'Authorization': 'Bearer ' + token },
            contentType: 'application/json',
            data: JSON.stringify({
                message: message,
                conversation_id: currentConversationId
            }),
            success: function (data) {
                removeLoadingMessage();
                currentConversationId = data.conversation_id;

                if (data.message) {
                    appendMessage(data.message.role, data.message.content);
                }

                // Reload conversation list
                loadConversations();
            },
            error: function (xhr) {
                removeLoadingMessage();
                var error = xhr.responseJSON ? xhr.responseJSON.error : 'Failed to send message';
                appendMessage('assistant', 'Error: ' + error);
            },
            complete: function () {
                isLoading = false;
                $('#send-btn').prop('disabled', true);
                $('#chat-input').focus();
            }
        });
    }

    function scrollToBottom() {
        var container = document.getElementById('chat-messages');
        if (container) {
            container.scrollTop = container.scrollHeight;
        }
    }

    function escapeHtml(text) {
        var div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

})(jQuery);
