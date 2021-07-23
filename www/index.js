import {unlock} from "povunlock";


let fileSelect = document.getElementById('file');
console.log('a');
fileSelect.addEventListener('change', (event) => {
    fileSelect.disabled = true;
    let reader = new FileReader();
    reader.readAsArrayBuffer(fileSelect.files[0]);
    reader.addEventListener('load', () => {
        console.log(reader.result);
        let unlocked = unlock(new Uint8Array(reader.result));
        fileSelect.disabled = false;
        save(unlocked, "unlocked.dem");
    });
});

function save(data, fileName) {
    let a = document.createElement("a");
    document.body.appendChild(a);
    a.style = "display: none";
    let blob = new Blob([data], {type: "octet/stream"});
    let url = window.URL.createObjectURL(blob);
    a.href = url;
    a.download = fileName;
    a.click();
    window.URL.revokeObjectURL(url);
}
