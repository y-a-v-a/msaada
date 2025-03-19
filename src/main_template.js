// Basic JavaScript for Msaada default page

// Wait for DOM to be fully loaded
document.addEventListener('DOMContentLoaded', function() {
    // Add a timestamp to show when the page was last loaded
    const footer = document.createElement('footer');
    footer.innerHTML = `Page loaded at: ${new Date().toLocaleString()}`;
    document.querySelector('.container').appendChild(footer);
    
    // Add a class to the paragraph to demonstrate CSS interaction
    const firstParagraph = document.querySelector('p');
    if (firstParagraph) {
        firstParagraph.classList.add('highlight');
    }
    
    // Console message for developers
    console.log('Msaada server is running. This page was generated automatically.');
    
    // Simple interaction demo
    const heading = document.querySelector('h1');
    if (heading) {
        heading.style.cursor = 'pointer';
        heading.addEventListener('click', function() {
            alert('Welcome to Msaada web server!');
        });
    }
    
    // Handle form submissions to test the POST API
    setupPostRequestTests();
});

function setupPostRequestTests() {
    // 1. Multipart Form Data with File Upload
    const fileUploadForm = document.getElementById('fileUploadForm');
    const submitFormBtn = document.getElementById('submitForm');
    const formResponse = document.getElementById('formResponse');
    
    if (fileUploadForm && submitFormBtn) {
        submitFormBtn.addEventListener('click', function() {
            const formData = new FormData(fileUploadForm);
            
            fetch('/form-test', {
                method: 'POST',
                body: formData
            })
            .then(response => {
                if (!response.ok) {
                    throw new Error(`HTTP error ${response.status}: ${response.statusText}`);
                }
                return response.json();
            })
            .then(data => {
                formResponse.textContent = JSON.stringify(data, null, 2);
            })
            .catch(error => {
                formResponse.textContent = `Error: ${error.message}`;
                console.error('Form submission error:', error);
            });
        });
    }
    
    // 2. JSON Data
    const jsonDataTextarea = document.getElementById('jsonData');
    const sendJsonButton = document.getElementById('sendJson');
    const jsonResponse = document.getElementById('jsonResponse');
    
    if (sendJsonButton && jsonDataTextarea) {
        sendJsonButton.addEventListener('click', function() {
            let jsonData;
            try {
                jsonData = JSON.parse(jsonDataTextarea.value);
            } catch (error) {
                jsonResponse.textContent = `Error parsing JSON: ${error.message}`;
                return;
            }
            
            fetch('/json-test', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify(jsonData)
            })
            .then(response => {
                if (!response.ok) {
                    throw new Error(`HTTP error ${response.status}: ${response.statusText}`);
                }
                return response.json();
            })
            .then(data => {
                jsonResponse.textContent = JSON.stringify(data, null, 2);
            })
            .catch(error => {
                jsonResponse.textContent = `Error: ${error.message}`;
                console.error('JSON submission error:', error);
            });
        });
    }
    
    // 3. Plain Text
    const plainTextTextarea = document.getElementById('plainText');
    const sendTextButton = document.getElementById('sendText');
    const textResponse = document.getElementById('textResponse');
    
    if (sendTextButton && plainTextTextarea) {
        sendTextButton.addEventListener('click', function() {
            const text = plainTextTextarea.value;
            
            fetch('/text-test', {
                method: 'POST',
                headers: {
                    'Content-Type': 'text/plain'
                },
                body: text
            })
            .then(response => {
                if (!response.ok) {
                    throw new Error(`HTTP error ${response.status}: ${response.statusText}`);
                }
                return response.json();
            })
            .then(data => {
                textResponse.textContent = JSON.stringify(data, null, 2);
            })
            .catch(error => {
                textResponse.textContent = `Error: ${error.message}`;
                console.error('Text submission error:', error);
            });
        });
    }
}