#{
  let display = eval(mode: "code", sys.inputs.at("d", default: "true"))
  let equation = eval(mode: "math", sys.inputs.at("eq", default: "quest.double"))
  let eq = math.equation(block: display, equation)
  set text(top-edge: "bounds", size: 12pt)
  show math.equation: set text(font: "IBM Plex Math")
  context {
    let rendered-tight = measure(eq).height
    set text(bottom-edge: "bounds")
    context {
      let rendered-loose = measure(eq).height
      let diff = rendered-loose - rendered-tight

      set page(
        width: auto,
        height: auto,
        margin: 0pt,
        fill: none,
      )
      [#metadata(diff.mm())<down>]
      eq
    }
  }
}
