// Static LaTeX vocabulary for autocompletion.

export const COMMANDS: [string, string][] = [
  ["section{}", "Section"], ["subsection{}", "Subsection"], ["subsubsection{}", "Subsubsection"], ["chapter{}", "Chapter"], ["paragraph{}", "Paragraph heading"],
  ["textbf{}", "Bold"], ["textit{}", "Italic"], ["emph{}", "Emphasis"], ["underline{}", "Underline"], ["texttt{}", "Typewriter"], ["textsc{}", "Small caps"],
  ["label{}", "Label for cross-references"], ["ref{}", "Reference"], ["eqref{}", "Equation reference (amsmath)"], ["pageref{}", "Page reference"], ["cref{}", "Smart reference (cleveref)"], ["autoref{}", "Auto reference (hyperref)"],
  ["cite{}", "Citation"], ["parencite{}", "Citation in parentheses (biblatex)"], ["textcite{}", "Textual citation (biblatex)"], ["citep{}", "Parenthetical citation (natbib)"], ["citet{}", "Textual citation (natbib)"],
  ["usepackage{}", "Load a package"], ["documentclass{}", "Document class"], ["input{}", "Include file"], ["include{}", "Include chapter"],
  ["includegraphics[width=\\linewidth]{}", "Image (graphicx)"], ["caption{}", "Caption"], ["centering", "Center content"],
  ["frac{}{}", "Fraction"], ["sqrt{}", "Square root"], ["sum_{}^{}", "Sum"], ["int_{}^{}", "Integral"], ["prod_{}^{}", "Product"], ["lim_{}", "Limit"],
  ["alpha", "α"], ["beta", "β"], ["gamma", "γ"], ["delta", "δ"], ["epsilon", "ϵ"], ["varepsilon", "ε"], ["lambda", "λ"], ["mu", "μ"], ["pi", "π"], ["sigma", "σ"], ["theta", "θ"], ["omega", "ω"], ["phi", "ϕ"], ["varphi", "φ"],
  ["infty", "∞"], ["partial", "∂"], ["nabla", "∇"], ["cdot", "·"], ["times", "×"], ["leq", "≤"], ["geq", "≥"], ["neq", "≠"], ["approx", "≈"], ["rightarrow", "→"], ["Rightarrow", "⇒"], ["mathbb{}", "Blackboard bold (amssymb)"], ["mathrm{}", "Upright math"], ["text{}", "Text in math (amsmath)"],
  ["item ", "List item"], ["footnote{}", "Footnote"], ["url{}", "URL"], ["href{}{}", "Link (hyperref)"], ["textcolor{}{}", "Coloured text (xcolor)"],
  ["maketitle", "Typeset title"], ["tableofcontents", "Table of contents"], ["listoffigures", "List of figures"], ["listoftables", "List of tables"], ["printbibliography", "Bibliography (biblatex)"], ["bibliography{}", "Bibliography (BibTeX)"], ["addbibresource{}", "Add .bib (biblatex)"],
  ["title{}", "Title"], ["author{}", "Author"], ["date{}", "Date"], ["today", "Today's date"],
  ["newpage", "New page"], ["clearpage", "Flush floats + new page"], ["vspace{}", "Vertical space"], ["hspace{}", "Horizontal space"], ["noindent", "No indent"], ["newline", "Line break"],
  ["toprule", "Top rule (booktabs)"], ["midrule", "Middle rule (booktabs)"], ["bottomrule", "Bottom rule (booktabs)"], ["hline", "Horizontal line"], ["multicolumn{}{}{}", "Multi-column cell"],
  ["newcommand{}{}", "Define command"], ["renewcommand{}{}", "Redefine command"], ["SI{}{}", "Quantity (siunitx)"],
];

export const ENVIRONMENTS: [string, string][] = [
  ["document", ""], ["itemize", "Bullet list"], ["enumerate", "Numbered list"], ["description", "Description list"],
  ["equation", "Numbered equation"], ["equation*", "Unnumbered equation"], ["align", "Aligned equations (amsmath)"], ["align*", "Aligned, unnumbered"], ["gather", "Gathered equations"], ["cases", "Case distinction"],
  ["pmatrix", "Matrix ( )"], ["bmatrix", "Matrix [ ]"],
  ["figure", "Figure float"], ["table", "Table float"], ["tabular", "Table"], ["center", "Centered"], ["minipage", "Mini page"],
  ["abstract", "Abstract"], ["quote", "Quote"], ["verbatim", "Verbatim"], ["lstlisting", "Code (listings)"], ["tikzpicture", "Drawing (TikZ)"],
  ["frame", "Beamer slide"], ["theorem", "Theorem"], ["proof", "Proof (amsthm)"],
];

/** Body snippets for \begin{…} completions. `|` marks the cursor. */
export const ENV_BODY: Record<string, string> = {
  itemize: "\t\\item |", enumerate: "\t\\item |", description: "\t\\item[|] ",
  figure: "\t\\centering\n\t\\includegraphics[width=0.8\\linewidth]{|}\n\t\\caption{}\n\t\\label{fig:}",
  table: "\t\\centering\n\t\\begin{tabular}{ll}\n\t\t\\toprule\n\t\t| & \\\\\n\t\t\\midrule\n\t\t & \\\\\n\t\t\\bottomrule\n\t\\end{tabular}\n\t\\caption{}\n\t\\label{tab:}",
  tabular: "\t|",
  frame: "\t\\frametitle{|}\n\t",
};
