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
});