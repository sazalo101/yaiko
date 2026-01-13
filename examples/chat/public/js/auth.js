/**
 * Yaiko Chat - Auth JS
 */
(function ($) {
    'use strict';

    $(document).ready(function () {
        // Check if already logged in
        var token = localStorage.getItem('token');
        if (token) {
            window.location.href = '/chat';
            return;
        }

        // Tab switching
        $('.auth-tab').on('click', function () {
            var tab = $(this).data('tab');
            $('.auth-tab').removeClass('active');
            $(this).addClass('active');

            if (tab === 'login') {
                $('#login-form').show();
                $('#signup-form').hide();
            } else {
                $('#login-form').hide();
                $('#signup-form').show();
            }
        });

        // Login form
        $('#login-form').on('submit', function (e) {
            e.preventDefault();

            var email = $('#login-email').val();
            var password = $('#login-password').val();

            $('#login-error').text('');

            $.ajax({
                url: '/api/login',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify({ email: email, password: password }),
                success: function (data) {
                    localStorage.setItem('token', data.token);
                    localStorage.setItem('user', JSON.stringify(data.user));
                    window.location.href = '/chat';
                },
                error: function (xhr) {
                    var error = xhr.responseJSON ? xhr.responseJSON.error : 'Login failed';
                    $('#login-error').text(error);
                }
            });
        });

        // Signup form
        $('#signup-form').on('submit', function (e) {
            e.preventDefault();

            var email = $('#signup-email').val();
            var password = $('#signup-password').val();

            $('#signup-error').text('');

            if (password.length < 6) {
                $('#signup-error').text('Password must be at least 6 characters');
                return;
            }

            $.ajax({
                url: '/api/signup',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify({ email: email, password: password }),
                success: function (data) {
                    localStorage.setItem('token', data.token);
                    localStorage.setItem('user', JSON.stringify(data.user));
                    window.location.href = '/chat';
                },
                error: function (xhr) {
                    var error = xhr.responseJSON ? xhr.responseJSON.error : 'Signup failed';
                    $('#signup-error').text(error);
                }
            });
        });
    });

})(jQuery);
