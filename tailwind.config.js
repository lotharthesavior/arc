/** @type {import('tailwindcss').Config} */
module.exports = {
  darkMode: 'class',
  content: ["./crates/arc-app/src/resources/**/*.{html,js}"],
  theme: {
    extend: {},
  },
  plugins: [
    require('@tailwindcss/forms'),
  ],
}
