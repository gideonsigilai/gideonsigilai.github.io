const projects={onekitty:{title:'OneKitty',category:'MOBILE & WEB / FINTECH',description:'A group finance application built around contributions, payments, and community. My work focuses on the mobile experience, connecting everyday financial tasks through a consistent Flutter interface.',features:['Transactions and exportable statements','Group management and regional payment integrations','KYC flows, multilingual experiences, and event ticket validation'],tags:['Flutter','GetX','Dio','GetStorage']},onechurch:{title:'OneChurch',category:'MOBILE & WEB / COMMUNITY',description:'A church management platform that brings member experiences and administration into one connected system. Built with Flutter across mobile and web.',features:['Member and administrator experiences','Events, ticketing, groups, and announcements','Donation records, attendance insights, and dashboards'],tags:['Flutter Web','GetX','Firebase','REST APIs']},plant:{title:'Plant Disease Detector',category:'APPLIED AI / COMPUTER VISION',description:'An exploration of image classification for plant health, connecting a trained machine learning model to a Flutter application for on-device inference.',features:['Image classification using TensorFlow and Keras','Dataset preparation and model evaluation','TensorFlow Lite conversion and Flutter integration'],tags:['Python','TensorFlow','TFLite','Flutter']}};
const $=s=>document.querySelector(s),$$=s=>[...document.querySelectorAll(s)];
const reduced=matchMedia('(prefers-reduced-motion: reduce)');
const dialog=$('#project-dialog'),keys=Object.keys(projects);let projectIndex=0;
function renderProject(key){projectIndex=keys.indexOf(key);const p=projects[key];$('#dialog-title').textContent=p.title;$('#dialog-category').textContent=p.category;$('#dialog-description').textContent=p.description;$('#dialog-features').replaceChildren(...p.features.map(text=>{const li=document.createElement('li');li.textContent=text;return li}));$('#dialog-tags').replaceChildren(...p.tags.map(text=>{const tag=document.createElement('span');tag.textContent=text;return tag}));$('#project-position').textContent=`${projectIndex+1} / ${keys.length}`;dialog.scrollTop=0;}
function openProject(key){renderProject(key);if(!dialog.open)dialog.showModal()}
$$('[data-project]').forEach(button=>button.addEventListener('click',()=>openProject(button.dataset.project)));
function stepProject(step){renderProject(keys[(projectIndex+step+keys.length)%keys.length])}
$('#previous-project').addEventListener('click',()=>stepProject(-1));$('#next-project').addEventListener('click',()=>stepProject(1));
dialog.addEventListener('keydown',e=>{if(e.key==='ArrowRight'){e.preventDefault();stepProject(1)}if(e.key==='ArrowLeft'){e.preventDefault();stepProject(-1)}});
$$('dialog').forEach(d=>{d.querySelector('.close').addEventListener('click',()=>d.close());d.addEventListener('click',event=>{if(event.target===d){const r=d.getBoundingClientRect();if(event.clientX<r.left||event.clientX>r.right||event.clientY<r.top||event.clientY>r.bottom)d.close()}})});
$$('[data-filter]').forEach(button=>button.addEventListener('click',()=>{$$('[data-filter]').forEach(b=>{b.classList.toggle('active',b===button);b.setAttribute('aria-pressed',String(b===button))});let count=0;$$('.project').forEach(p=>{p.hidden=button.dataset.filter!=='all'&&p.dataset.category!==button.dataset.filter;if(!p.hidden){count++;if(!reduced.matches)p.animate([{opacity:.35,transform:'translateY(12px)'},{opacity:1,transform:'translateY(0)'}],{duration:350,easing:'ease-out'})}});$('.filter-status').textContent=`Showing ${count} projects`;}));
const tools={Flutter:['One codebase, carefully adapted to mobile and web. Flutter is at the heart of my product development.','onekitty'],Dart:['The language behind my Flutter work, connecting interface logic, asynchronous operations, and application models.','onekitty'],GetX:['State management, dependency injection, and navigation to keep complex application flows organized.','onechurch'],'Responsive UI':['Layouts that adapt to the space available, from a phone in one hand to a full desktop workspace.','onechurch'],Go:['A focus for backend development: clear server logic, API design, and dependable services.',null],Firebase:['Cloud services used across my projects, including hosting Flutter web experiences.','onechurch'],Supabase:['A relational approach to application data, combining PostgreSQL with backend services.',null],'REST APIs':['Connecting interfaces to real data, with attention to loading states, errors, and predictable data flows.','onekitty'],Python:['A practical workspace for preparing datasets, training models, and evaluating results.','plant'],TensorFlow:['Building and training image classification models for plant disease detection.','plant'],TFLite:['Bringing trained models onto a device, connecting machine learning with a usable mobile experience.','plant'],Git:['Version control for tracking changes and collaborating throughout the development process.',null]};
let relatedProject=null;$$('.skill-tags button').forEach(button=>button.addEventListener('click',()=>{const [description,key]=tools[button.textContent];$$('.skill-tags button').forEach(b=>b.setAttribute('aria-pressed',String(b===button)));$('#tool-name').textContent=button.textContent;$('#tool-description').textContent=description;relatedProject=key;$('#tool-project').hidden=!key;if(key)$('#tool-project').textContent=`Explore ${projects[key].title} ＋`;if(!reduced.matches)$('#tool-explorer').animate([{opacity:.5,transform:'translateY(5px)'},{opacity:1,transform:'translateY(0)'}],{duration:250});}));$('#tool-project').addEventListener('click',()=>{if(relatedProject)openProject(relatedProject)});
const contact=$('#contact-dialog');$('#contact-open').addEventListener('click',()=>contact.showModal());$('#contact-done').addEventListener('click',()=>contact.close());$('#year').textContent=new Date().getFullYear();
const sections=$$('main>section[id]'),navLinks=$$('header a[href^="#"]');let queued=false;
function updateScroll(){const max=document.documentElement.scrollHeight-innerHeight;$('.reading-progress').style.transform=`scaleX(${max>0?scrollY/max:0})`;let current='';sections.forEach(s=>{if(s.getBoundingClientRect().top<=160)current=s.id});navLinks.forEach(a=>{const active=a.hash===`#${current}`;a.classList.toggle('nav-active',active);if(active)a.setAttribute('aria-current','location');else a.removeAttribute('aria-current')});queued=false;}
addEventListener('scroll',()=>{if(!queued){queued=true;requestAnimationFrame(updateScroll)}},{passive:true});addEventListener('resize',updateScroll);updateScroll();
if('IntersectionObserver' in window){const reveal=new IntersectionObserver(entries=>entries.forEach(e=>{if(e.isIntersecting){if(!reduced.matches)e.target.animate([{opacity:.2,transform:'translateY(24px)'},{opacity:1,transform:'translateY(0)'}],{duration:650,easing:'cubic-bezier(.2,.7,.2,1)'});reveal.unobserve(e.target)}}),{threshold:.12});$$('.project,.about-grid,.skills>div,.contact').forEach(el=>reveal.observe(el))}
const fine=matchMedia('(hover: hover) and (pointer: fine)');$$('.project-art').forEach(card=>{card.addEventListener('pointermove',e=>{if(!fine.matches||reduced.matches)return;const r=card.getBoundingClientRect(),x=(e.clientX-r.left)/r.width-.5,y=(e.clientY-r.top)/r.height-.5;card.style.transform=`perspective(1100px) rotateX(${-y*5}deg) rotateY(${x*5}deg)`;card.style.setProperty('--spot-x',`${(x+.5)*100}%`);card.style.setProperty('--spot-y',`${(y+.5)*100}%`)});card.addEventListener('pointerleave',()=>card.style.transform='')});
$('.spin-trigger').addEventListener('click',()=>{if(reduced.matches)return;$('.spin-trigger').animate([{transform:'rotate(0)'},{transform:'rotate(360deg)'}],{duration:950,easing:'cubic-bezier(.2,.8,.2,1)'})});

// Scroll depth uses independent CSS translation so hover tilt and click rotations coexist.
(() => {
  const compact = matchMedia('(max-width: 700px)');
  const layers = [
    { selector: '.hero .asterisk', anchor: '.hero', speed: .17, limit: 48, hero: true },
    { selector: '.hero h1 .serif', anchor: '.hero', speed: .045, limit: 13, hero: true },
    { selector: '.photo-frame', anchor: '.project', speed: -.11, limit: 30 },
    { selector: '.small-placeholder', anchor: '.project', speed: -.10, limit: 25 },
    { selector: '.art-title', anchor: '.project', speed: .045, limit: 12 },
    { selector: '.portrait-initials', anchor: '.about', speed: -.10, limit: 32 },
    { selector: '.contact-star', anchor: '.contact', speed: -.12, limit: 25 }
  ].flatMap(config => $$(config.selector).map(element => {
    element.classList.add('parallax-layer');
    return { ...config, element, anchor: element.closest(config.anchor) };
  }));
  let frame = 0;
  function paint() {
    frame = 0;
    if (reduced.matches) {
      layers.forEach(({ element }) => element.style.removeProperty('--parallax-y'));
      return;
    }
    const amplitude = compact.matches ? .4 : 1;
    const bounds = new Map();
    // Read layout first, then update only compositor-friendly translations.
    layers.forEach(({ anchor }) => {
      if (!bounds.has(anchor)) bounds.set(anchor, anchor.getBoundingClientRect());
    });
    layers.forEach(({ element, anchor, hero, speed, limit }) => {
      const rect = bounds.get(anchor);
      if (!rect.height || rect.bottom < -100 || rect.top > innerHeight + 100) return;
      const distance = hero ? Math.max(0, scrollY) : innerHeight / 2 - (rect.top + rect.height / 2);
      const offset = Math.max(-limit, Math.min(limit, distance * speed)) * amplitude;
      element.style.setProperty('--parallax-y', `${offset.toFixed(2)}px`);
    });
  }
  function schedule() {
    if (!frame) frame = requestAnimationFrame(paint);
  }
  addEventListener('scroll', schedule, { passive: true });
  addEventListener('resize', schedule);
  addEventListener('pageshow', schedule);
  reduced.addEventListener('change', schedule);
  compact.addEventListener('change', schedule);
  $$('[data-filter]').forEach(button => button.addEventListener('click', schedule));
  if ('ResizeObserver' in window) new ResizeObserver(schedule).observe(document.querySelector('main'));
  if (document.fonts) document.fonts.ready.then(schedule);
  schedule();
})();
