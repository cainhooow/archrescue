'use strict';
const dialog = document.getElementById('viewer');
const nodes = Object.fromEntries(['viewer-section','viewer-title','viewer-image','viewer-caption','viewer-count','download','source','close','previous','next'].map(id=>[id,document.getElementById(id)]));
let sectionIndex=0, shotIndex=0, opener;
function display(){
  const section=window.gallery[sectionIndex];
  const shot=section.shots[shotIndex];
  nodes['viewer-section'].textContent=section.name;
  nodes['viewer-title'].textContent=shot.label;
  nodes['viewer-image'].src=shot.file;
  nodes['viewer-image'].alt='Mockup '+shot.label;
  nodes['viewer-caption'].textContent=shot.description;
  nodes['viewer-count'].textContent=(shotIndex+1)+' / '+section.shots.length;
  nodes.download.href=shot.file;
  nodes.source.href=shot.sourceUrl;
  nodes.previous.disabled=shotIndex===0&&sectionIndex===0;
  nodes.next.disabled=sectionIndex===window.gallery.length-1&&shotIndex===section.shots.length-1;
}
function move(step){
  const requested=shotIndex+step;
  if(requested>=0&&requested<window.gallery[sectionIndex].shots.length){
    shotIndex=requested;
  }else{
    const section=sectionIndex+step;
    if(section<0||section>=window.gallery.length)return;
    sectionIndex=section;
    shotIndex=step<0?window.gallery[sectionIndex].shots.length-1:0;
  }
  display();
}
document.querySelectorAll('[data-section]').forEach(button=>button.addEventListener('click',()=>{
  opener=button;
  sectionIndex=Number(button.dataset.section);
  shotIndex=Number(button.dataset.index);
  display();
  dialog.showModal();
  nodes.close.focus();
}));
nodes.close.addEventListener('click',()=>dialog.close());
nodes.previous.addEventListener('click',()=>move(-1));
nodes.next.addEventListener('click',()=>move(1));
dialog.addEventListener('keydown',event=>{
  if(event.key==='ArrowLeft'||event.key==='ArrowRight'){
    event.preventDefault();
    move(event.key==='ArrowLeft'?-1:1);
  }
});
dialog.addEventListener('close',()=>opener?.focus());
