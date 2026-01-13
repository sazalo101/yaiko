/**
 * Yaiko Core JavaScript
 */
(function($) {
    'use strict';
    
    window.Yaiko = window.Yaiko || {};
    
    Yaiko.api = {
        baseUrl: '',
        getCsrfToken: function() {
            return $('meta[name="csrf-token"]').attr('content') || '';
        },
        request: function(method, url, data, options) {
            var defaults = {
                method: method,
                url: this.baseUrl + url,
                contentType: 'application/json',
                dataType: 'json',
                headers: { 'X-CSRF-Token': this.getCsrfToken() }
            };
            if (data && (method === 'POST' || method === 'PUT' || method === 'PATCH')) {
                defaults.data = JSON.stringify(data);
            }
            var settings = $.extend({}, defaults, options);
            return $.ajax(settings).fail(function(xhr, status, error) {
                var message = xhr.responseJSON ? xhr.responseJSON.message : (error || 'An error occurred');
                Yaiko.ui.toast(message, 'error');
            });
        },
        get: function(url, options) { return this.request('GET', url, null, options); },
        post: function(url, data, options) { return this.request('POST', url, data, options); },
        put: function(url, data, options) { return this.request('PUT', url, data, options); },
        delete: function(url, options) { return this.request('DELETE', url, null, options); }
    };
    
    Yaiko.ui = {
        toast: function(message, type) {
            type = type || 'info';
            var $container = $('#toast-container');
            var $toast = $('<div>').addClass('toast toast--' + type).text(message);
            $container.append($toast);
            setTimeout(function() { $toast.fadeOut(300, function() { $(this).remove(); }); }, 5000);
        },
        showLoading: function($el) { $el.addClass('loading').prop('disabled', true); },
        hideLoading: function($el) { $el.removeClass('loading').prop('disabled', false); },
        modal: {
            open: function(id) { $('#' + id).addClass('is-active'); $('body').addClass('modal-open'); },
            close: function(id) { $('#' + id).removeClass('is-active'); $('body').removeClass('modal-open'); }
        }
    };
    
    Yaiko.utils = {
        formatDate: function(date) { return new Date(date).toLocaleDateString(); },
        debounce: function(func, wait) {
            var timeout;
            return function() {
                var context = this, args = arguments;
                clearTimeout(timeout);
                timeout = setTimeout(function() { func.apply(context, args); }, wait);
            };
        }
    };
    
    $(document).ready(function() { console.log('🚀 Yaiko initialized'); });
})(jQuery);
