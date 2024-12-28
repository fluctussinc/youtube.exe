function checkInternetConnection() {
    return new Promise((resolve) => {
        fetch('https://www.google.com', {
            method: 'GET',
            mode: 'no-cors',
            cache: 'no-store'
        })
        .then(() => resolve(true))
        .catch(() => resolve(false));
    });
}

function navigateToMainUrl() {
    const startupMode = new URLSearchParams(window.location.search).get('startup') === 'true';
    const targetUrl = startupMode ? 
        "https://youtu.be/" :
        "https://youtu.be/";
    
    window.location.href = targetUrl;
}

function displayError(message) {
    const loadingContainer = document.getElementById('loading-container');
    const errorContainer = document.getElementById('error-container');
    const errorMessageEl = document.getElementById('error-message');

    loadingContainer.classList.add('hidden');
    errorMessageEl.textContent = message;
    errorMessageEl.classList.add('show');

    alert(message);
}

let lastSubmissionCount = null;

async function checkForNewSubmissions() {
    try {
        const hasInternet = await checkInternetConnection();
        if (!hasInternet) {
            console.error('No internet connection during submission check');
            return;
        }

        const response = await fetch(window.location.href);
        const text = await response.text();
        const parser = new DOMParser();
        const doc = parser.parseFromString(text, 'text/html');
        
        // Adjust selector based on your submissions table
        const submissions = doc.querySelectorAll('table tr').length - 1;
        
        if (lastSubmissionCount !== null && submissions > lastSubmissionCount) {
            const newSubmissions = submissions - lastSubmissionCount;
            window.ipc.postMessage(JSON.stringify({
                notification: {
                    title: "New Submission",
                    message: `${newSubmissions} new submission(s) received!`
                }
            }));
        }
        
        lastSubmissionCount = submissions;
    } catch (error) {
        console.error('Error checking submissions:', error);
    }
}

function startSubmissionMonitoring() {
    // Initial check
    checkForNewSubmissions();
    
    // Check every 30 seconds
    setInterval(checkForNewSubmissions, 30000);
}

document.addEventListener('DOMContentLoaded', async () => {
    try {
        const hasInternet = await checkInternetConnection();
        if (!hasInternet) {
            displayError('No Internet Connection');
            return;
        }

        // If we're on the submissions page, start monitoring
        if (window.location.href.includes('submissions.php')) {
            startSubmissionMonitoring();
        } else {
            // Otherwise navigate to the appropriate page
            setTimeout(navigateToMainUrl, 1000);
        }
    } catch {
        displayError('Connection Check Failed');
    }
});

// Listen for page changes to start monitoring when reaching submissions page
window.addEventListener('load', () => {
    if (window.location.href.includes('submissions.php')) {
        startSubmissionMonitoring();
    }
});