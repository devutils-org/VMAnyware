// Shared owned acceptance helper; performs only explicit real UI stored-detail reads.
window.__checkRetainedDetail = async () => {
  const expected=window.__snapshotFixture.detailInstance;
  const panel=document.querySelector(`.remote-instance-detail[data-instance-id="${expected.id}"]`);
  const check=(ok,message)=>{if(!ok)throw Error(message);};
  check(!!panel,'Selected retained-detail panel missing');
  const button=label=>[...panel.querySelectorAll('button')].find(node=>node.textContent===label);
  async function wait(predicate,label){const end=Date.now()+15000;while(!predicate()){if(Date.now()>end)throw Error('Native detail wait expired: '+label+'; '+panel.textContent);await new Promise(resolve=>setTimeout(resolve,20));}}
  async function click(label){await wait(()=>button(label)&&!button(label).disabled,label);button(label).click();}
  check(!panel.querySelector('pre'),'Detail loaded without an explicit read');
  await click('Read definition for VM 202');
  await wait(()=>panel.querySelector('pre')&&panel.getAttribute('aria-busy')==='false','saved definition');
  function validate(){
    check(panel.querySelector('pre').textContent===expected.definitionYaml,'Retained YAML bytes changed in display');
    check(panel.querySelector('p code').textContent===expected.definitionHash,'Retained hash changed');
    check(panel.textContent.includes('Definition revision: '+expected.definitionRevision),'Retained revision changed');
    check(panel.textContent.includes('Retained definition: '+expected.definitionName),'Retained definition name changed');
    check(panel.textContent.includes(expected.status==='deleted'?'deleted · definition retained':'Association: registered'),'Retained association status changed');
    check(panel.textContent.includes('saved configuration declaration')&&panel.textContent.includes('does not verify current VM resources'),'Declaration presented as observed resources');
    check(!panel.querySelector('input')&&![...panel.querySelectorAll('button')].some(node=>/confirm|SSH/.test(node.textContent)),'Read offered consent or SSH');
  }
  validate();await click('Refresh definition for VM 202');await wait(()=>panel.querySelector('pre')&&panel.getAttribute('aria-busy')==='false','saved refresh');validate();
  await click('Hide saved definition for VM 202');await wait(()=>!panel.querySelector('pre'),'hide');
};
