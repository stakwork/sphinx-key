pub const HTML: &str = r#"
<!DOCTYPE html>
<html lang="en">

<head>
  <meta name="description" content="sphinxkey" />
  <meta charset="utf-8">
  <title>Sphinx Key</title>
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <link rel="shortcut icon" type="image/x-icon" href="data:image/x-icon;,"> 
  <style>
    html {
      font-family: Arial, Helvetica, sans-serif;
      color: white;
      font-size: 20px;
    }
    body{
      padding:0;
      background: #292a2d;
      display: flex;
      flex-direction: column;
      align-items: center;
    }
    #logo{
      margin-top:25px;
    }
    input {
      margin-top:10px;
      height:32px;
      width:230px;
      padding-left:15px;
      border-radius: 16px;
    }
    button {
      margin-top:20px;
      height:40px;
      width:108px;
      color:white;
      font-weight: bold;
      border-radius: 20px;
      background: #618AFF;
      cursor: pointer;
    }
    button:disabled {
      cursor:default;
      background:grey;
    }
    @keyframes spin {
      from {
        transform:rotate(0deg);
      }
      to {
        transform:rotate(360deg);
      }
    }
    #button svg {
      animation-name: spin;
      animation-duration: 600ms;
      animation-iteration-count: infinite;
      animation-timing-function: linear; 
    }
    #loading {
      display: none;
    }
  </style> 
</head>

<body>

  <svg id="logo" width="194px" height="186px" viewBox="0 0 194 186" version="1.1" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
    <path fill="white" transform="translate(-508.000000, -189.000000)" stroke-width="1" d="M667.21,214.51 C658.045169,206.151145 647.39915,199.578356 635.82,195.13 C625.907772,191.21621 615.366017,189.139033 604.71,189 L604.64,189 C593.983983,189.139033 583.442228,191.21621 573.53,195.13 C561.95085,199.578356 551.304831,206.151145 542.14,214.51 C526.56,228.7 508,254.87 508,299.01 C508.002384,301.229806 509.18759,303.280098 511.11,304.39 C511.3,304.5 529.85,315.39 533.38,336.77 C535.8,351.44 545.74,362.71 562.12,369.36 C569.522132,372.317389 577.318819,374.170534 585.26,374.86 C586.972704,374.964831 588.652634,374.357474 589.902326,373.181627 C591.152018,372.00578 591.860447,370.36591 591.86,368.65 L591.86,324.52 C595.931833,325.717067 600.155885,326.316653 604.4,326.3 C608.837411,326.319158 613.252794,325.675389 617.5,324.39 L617.5,368.65 C617.5,370.299647 618.15532,371.881728 619.321796,373.048204 C620.488272,374.21468 622.070353,374.87 623.72,374.87 L624.1,374.87 C632.041181,374.180534 639.837868,372.327389 647.24,369.37 C663.6,362.72 673.54,351.45 676,336.77 C677.588215,327.719728 681.863657,319.357004 688.27,312.77 C691.228816,309.557963 694.589929,306.74135 698.27,304.39 C700.19241,303.280098 701.377616,301.229806 701.38,299.01 C701.34,254.87 682.78,228.7 667.21,214.51 Z M604.64,201.43 L604.71,201.43 C617.38,201.43 635.81,205.99 652.35,218.35 L641.43,239.24 L567.63,239.24 L556.93,218.4 C573.49,206 592,201.43 604.64,201.43 Z M545.65,334.77 C542.12,313.39 527.11,300.48 520.48,295.72 C521.29,261.14 534.75,239.52 547.28,226.83 L557.6,246.93 L557.65,273.15 C557.6448,282.683193 559.914304,292.08004 564.27,300.56 C567.824721,307.643965 573.02275,313.774622 579.43,318.44 L579.43,361.54 C568.77,359.48 548.7,353.22 545.65,334.77 Z M575.3,294.77 C571.858657,288.054866 570.069114,280.615578 570.08,273.07 L570.079951,251.64 L639,251.64 L638.81,272.13 C638.761448,279.909114 636.94573,287.575481 633.5,294.55 C629.09,303.34 620.5,313.83 604.44,313.83 C587.69,313.87 579,301.93 575.3,294.77 Z M663.69,334.77 C662,345.01 654.98,352.77 642.82,357.77 C638.655184,359.452348 634.334139,360.718515 629.92,361.55 L629.92,318.12 C636.158361,313.363256 641.179183,307.194329 644.57,300.12 C648.863933,291.449073 651.131029,281.915644 651.2,272.24 L651.43,246.92 L662,226.77 C674.55,239.45 688.06,261.08 688.87,295.77 C682.23,300.46 667.22,313.37 663.69,334.77 Z" id="Shape"></path>
  </svg>

  <p style="max-width:260px;text-align:center;margin-top:32px;">
    Enter your WiFi credentials and MQTT Broker to connect you Sphinx Key
  </p>

  <input id="ssid" placeholder="WiFi SSID" />

  <input id="pass" placeholder="Password" />

  <input id="broker" placeholder="Broker" />

  <button id="button" type="submit">
    <svg id="loading" height="16" width="16" viewbox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg">
      <path fill="white" stroke="grey" d="M988 548c-19.9 0-36-16.1-36-36 0-59.4-11.6-117-34.6-171.3a440.45 440.45 0 0 0-94.3-139.9 437.71 437.71 0 0 0-139.9-94.3C629 83.6 571.4 72 512 72c-19.9 0-36-16.1-36-36s16.1-36 36-36c69.1 0 136.2 13.5 199.3 40.3C772.3 66 827 103 874 150c47 47 83.9 101.8 109.7 162.7 26.7 63.1 40.2 130.2 40.2 199.3.1 19.9-16 36-35.9 36z" />
    </svg>
    <span style="margin:0 8px;">OK</span>
  </button>

  <p id="msg" style="max-width:260px;text-align:center;margin-top:32px;"></p>

</body>
<script>
function get(id){
  return document.getElementById(id)
}
const params = getParams()

const button = get('button')
button.disabled = true
const ssid = get('ssid')
const pass = get('pass')
const broker = get('broker')
const loading = get('loading')
const msg = get('msg')

if(params['ssid']) {
  ssid.value = params.ssid
}
if(params['pass']) {
  pass.value = params.pass
}
if(params['broker']) {
  broker.value = params.broker
}

function checker() {
  if(ssid.value && broker.value && button.disabled) {
    button.disabled = false
  } else if(!ssid.value || !broker.value) {
    button.disabled = true
  }
}
ssid.onchange = function(){
  checker()
}
ssid.oninput = function(){
  checker()
}
broker.onchange = function(){
  checker()
}
broker.oninput = function(){
  checker()
}

button.onclick = function(e) {
  fetchWithTimeout('/config?config=' +
    encodeURIComponent(JSON.stringify({
      ssid: ssid.value,
      pass: pass.value || '',
      broker: broker.value,
    })), {
      method:'POST',
      timeout:3000,
  })
  .then(r=> r.json())
  .then(finish)
  .catch(e=> {
    console.log(e)
    msg.innerHTML = 'Failed to post configuration to the Sphinx Key'
  })
}
async function finish(r) {
  console.log("done!", r)
  loading.style.display = 'inline-block'
  button.disabled = true
  await sleep(2000)
  loading.style.display = 'none'
  button.disabled = false
  msg.innerHTML = 'Please unplug your Sphinx Key and plug it back in'
}

function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms))
}

function getParams() {
  const ps = new URLSearchParams(window.location.search);
  const r = {};
  for (const [k, v] of ps) {
    r[k] = v;
  }
  return r;
}

async function fetchWithTimeout(resource, options = {}) {
  const { timeout = 5000 } = options;
  const controller = new AbortController();
  const id = setTimeout(() => controller.abort(), timeout);
  const response = await fetch(resource, {
    ...options,
    signal: controller.signal  
  });
  clearTimeout(id);
  return response;
}
</script>
</html>
"#;
