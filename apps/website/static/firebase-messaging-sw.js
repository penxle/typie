importScripts(
  'https://www.gstatic.com/firebasejs/12.18.0/firebase-app-compat.js',
  'https://www.gstatic.com/firebasejs/12.18.0/firebase-messaging-compat.js',
);

firebase.initializeApp({
  apiKey: 'AIzaSyB6JLO1FFETp7b10tjbzPBTPSYDLMee6aY',
  authDomain: 'typie-co.firebaseapp.com',
  projectId: 'typie-co',
  messagingSenderId: '378927208010',
  appId: '1:378927208010:web:dc8b55e5c05e9f6634b8b3',
});

firebase.messaging();
