/**
 * Application JavaScript
 */
$(document).ready(function() {
    $('#get-started').on('click', function() {
        Yaiko.ui.toast('Welcome to Yaiko! 🚀 Run "yaiko generate controller <name>" to add routes.', 'success');
    });
    
    $('#view-docs').on('click', function(e) {
        e.preventDefault();
        Yaiko.ui.toast('Opening Yaiko Documentation... 📚', 'info');
        setTimeout(function() {
            window.open('https://github.com/sazalo101/yaiko#readme', '_blank');
        }, 300);
    });
});
