// Msaada POST test script

// Detects the server URL dynamically
const serverUrl = window.location.origin;
console.log(`Server detected at: ${serverUrl}`);

// Format JSON for display
function formatJSON(json) {
    try {
        if (typeof json === 'string') {
            json = JSON.parse(json);
        }
        return JSON.stringify(json, null, 2);
    } catch (e) {
        return json;
    }
}

// Handle errors nicely
function handleError(element, error) {
    element.classList.add('error');
    element.textContent = `Error: ${error.message || error}`;
    console.error(error);
}

// Show success response
function showSuccess(element, data) {
    element.classList.add('success');
    element.textContent = formatJSON(data);
}

// Test multipart form data submission
document.getElementById('submitFormBtn').addEventListener('click', async () => {
    const form = document.getElementById('testForm');
    const resultEl = document.getElementById('formResult');
    
    try {
        resultEl.textContent = 'Sending...';
        resultEl.className = 'result-box';
        
        const formData = new FormData(form);
        
        // Send the request with absolute path
        const response = await fetch(`${serverUrl}/api/test-form`, {
            method: 'POST',
            body: formData
        });
        
        if (!response.ok) {
            throw new Error(`HTTP error ${response.status}: ${response.statusText}`);
        }
        
        const data = await response.json();
        showSuccess(resultEl, data);
    } catch (error) {
        handleError(resultEl, error);
    }
});

// Test JSON submission
document.getElementById('submitJsonBtn').addEventListener('click', async () => {
    const jsonInput = document.getElementById('jsonData');
    const resultEl = document.getElementById('jsonResult');
    
    try {
        resultEl.textContent = 'Sending...';
        resultEl.className = 'result-box';
        
        // Parse the JSON first to validate it
        const jsonData = JSON.parse(jsonInput.value);
        
        // Send the request with absolute path
        const response = await fetch(`${serverUrl}/api/test-json`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(jsonData)
        });
        
        if (!response.ok) {
            throw new Error(`HTTP error ${response.status}: ${response.statusText}`);
        }
        
        const data = await response.json();
        showSuccess(resultEl, data);
    } catch (error) {
        handleError(resultEl, error);
    }
});

// Test text submission
document.getElementById('submitTextBtn').addEventListener('click', async () => {
    const textInput = document.getElementById('textData');
    const resultEl = document.getElementById('textResult');
    
    try {
        resultEl.textContent = 'Sending...';
        resultEl.className = 'result-box';
        
        // Send the request with absolute path
        const response = await fetch(`${serverUrl}/api/test-text`, {
            method: 'POST',
            headers: {
                'Content-Type': 'text/plain'
            },
            body: textInput.value
        });
        
        if (!response.ok) {
            throw new Error(`HTTP error ${response.status}: ${response.statusText}`);
        }
        
        const data = await response.json();
        showSuccess(resultEl, data);
    } catch (error) {
        handleError(resultEl, error);
    }
});

// Display server information
document.addEventListener('DOMContentLoaded', () => {
    // Create a server info section
    const container = document.querySelector('.container');
    const infoDiv = document.createElement('div');
    infoDiv.className = 'server-info';
    infoDiv.style.marginBottom = '20px';
    infoDiv.style.padding = '10px';
    infoDiv.style.backgroundColor = '#e8f4f8';
    infoDiv.style.borderRadius = '4px';
    infoDiv.style.fontSize = '14px';
    
    infoDiv.innerHTML = `
        <strong>Server URL:</strong> ${serverUrl}<br>
        <strong>Test endpoints:</strong><br>
        - Form data: ${serverUrl}/api/test-form<br>
        - JSON: ${serverUrl}/api/test-json<br>
        - Text: ${serverUrl}/api/test-text
    `;
    
    // Insert after the first paragraph
    const firstP = container.querySelector('p');
    firstP.parentNode.insertBefore(infoDiv, firstP.nextSibling);
});