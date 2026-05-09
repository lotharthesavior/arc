/** @type {import('tailwindcss').Config} */
module.exports = {
  darkMode: 'class',
  content: ["./crates/nineties-app/src/resources/**/*.{html,js}"],
  theme: {
    extend: {},
  },
  plugins: [
    require('@tailwindcss/forms'),
  ],
}
