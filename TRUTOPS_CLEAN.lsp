;;; TRUTOPS_CLEAN.lsp
;;; Upraszcza polilinie (usuwa nadmiarowe wierzchołki) przed eksportem do TruTops.
;;; Działa w AutoCAD LT 2024+ (AutoLISP) oraz pełnym AutoCAD.
;;;
;;; Użycie:
;;;   (load "TRUTOPS_CLEAN.lsp")
;;;   TRUTOPS-CLEAN      — upraszcza polilinie w bieżącym rysunku
;;;   TRUTOPS-EXPORT     — zapisuje plik DXF

(defun trutops:dist2d (p1 p2)
  (sqrt (+ (expt (- (car p1) (car p2)) 2) (expt (- (cadr p1) (cadr p2)) 2)))
)

(defun trutops:dedupe (pts / out p)
  (setq out (list (car pts)))
  (foreach p (cdr pts)
    (if (> (trutops:dist2d p (car (reverse out))) 1e-6)
      (setq out (append out (list p)))
    )
  )
  out
)

(defun trutops:pt-seg-dist (p a b / ax ay bx by dx dy len t projx projy)
  (setq ax (car a) ay (cadr a) bx (car b) by (cadr b))
  (setq dx (- bx ax) dy (- by ay))
  (setq len (+ (* dx dx) (* dy dy)))
  (if (< len 1e-20)
    (trutops:dist2d p a)
    (progn
      (setq t (max 0.0 (min 1.0 (/ (+ (* (- (car p) ax) dx) (* (- (cadr p) ay) dy)) len))))
      (setq projx (+ ax (* t dx)) projy (+ ay (* t dy)))
      (sqrt (+ (expt (- (car p) projx) 2) (expt (- (cadr p) projy) 2)))
    )
  )
)

(defun trutops:take (pts n / r)
  (setq r '())
  (while (and (> n 0) pts)
    (setq r (append r (list (car pts))) pts (cdr pts) n (1- n))
  )
  r
)

;;; Ramer–Douglas–Peucker
(defun trutops:rdp (pts tol / start end i maxd idx d left right)
  (cond
    ((<= (length pts) 2) pts)
    (T
      (setq start (car pts) end (last pts) maxd 0.0 idx 0 i 1)
      (while (< i (1- (length pts)))
        (if (> (setq d (trutops:pt-seg-dist (nth i pts) start end)) maxd)
          (setq maxd d idx i)
        )
        (setq i (1+ i))
      )
      (if (<= maxd tol)
        (list start end)
        (progn
          (setq left (trutops:rdp (trutops:take pts (1+ idx)) tol))
          (setq right (trutops:rdp (nthcdr idx pts) tol))
          (append (reverse (cdr (reverse left))) right)
        )
      )
    )
  )
)

(defun trutops:get-pline-pts (ename / ent typ pts v ve closed layer)
  (setq ent (entget ename) typ (cdr (assoc 0 ent)) pts '())
  (cond
    ((= typ "LWPOLYLINE")
      (foreach pair ent
        (if (= (car pair) 10)
          (setq pts (append pts (list (list (cadr pair) (caddr pair)))))
        )
      )
      (setq closed (= (logand (cdr (assoc 70 ent)) 1) 1)
            layer (cdr (assoc 8 ent)))
      (list pts closed layer)
    )
    ((= typ "POLYLINE")
      (setq v (entnext ename) closed (= (logand (cdr (assoc 70 ent)) 1) 1)
            layer (cdr (assoc 8 ent)))
      (while (and v (/= "SEQEND" (cdr (assoc 0 (setq ve (entget v))))))
        (if (= (cdr (assoc 0 ve)) "VERTEX")
          (setq pts (append pts (list (list (cdr (assoc 10 ve)) (cdr (assoc 20 ve))))))
        )
        (setq v (entnext v))
      )
      (list pts closed layer)
    )
    (T nil)
  )
)

(defun trutops:make-lwpoly (pts closed layer / data)
  (setq data (list '(0 . "LWPOLYLINE") '(100 . "AcDbEntity") (cons 8 layer)
                   '(100 . "AcDbPolyline") (cons 90 (length pts))
                   (cons 70 (if closed 1 0))))
  (foreach p pts
    (setq data (append data (list (cons 10 (car p)) (cons 20 (cadr p)))))
  )
  (entmake data)
)

(defun c:TRUTOPS-CLEAN (/ tol ss i en data pts closed layer simp n-old n-new
                         total-old total-new)
  (princ "\n=== TRUTOPS-CLEAN — upraszczanie polilinii ===")
  (setq tol (getdist "\nTolerancja odchylenia w mm <0.1>: "))
  (if (not tol) (setq tol 0.1))
  (setq ss (ssget "_X" '((0 . "POLYLINE,LWPOLYLINE"))))
  (if (not ss)
    (princ "\nBrak polilinii w rysunku.")
    (progn
      (setq total-old 0 total-new 0 i 0)
      (repeat (sslength ss)
        (setq en (ssname ss i) data (trutops:get-pline-pts en))
        (if data
          (progn
            (setq pts (car data) closed (cadr data) layer (caddr data)
                  n-old (length pts)
                  simp (trutops:rdp (trutops:dedupe pts) tol)
                  n-new (length simp))
            (if (and closed (> n-new 2)
                     (< (trutops:dist2d (car simp) (last simp)) 1e-4))
              (setq simp (reverse (cdr (reverse simp))))
            )
            (entdel en)
            (trutops:make-lwpoly simp closed layer)
            (setq total-old (+ total-old n-old) total-new (+ total-new n-new))
          )
        )
        (setq i (1+ i))
      )
      (princ (strcat "\nGotowe. Wierzchołki: " (itoa total-old) " -> " (itoa total-new)
                     " (tolerancja " (rtos tol 2 4) ")"))
      (princ "\nUżyj TRUTOPS-EXPORT aby zapisać DXF.")
    )
  )
  (princ)
)

(defun c:TRUTOPS-EXPORT (/ f)
  (setq f (getfiled "Zapisz DXF dla TruTops" "" "dxf" 1))
  (if f
    (progn
      (command "_.-SAVEAS" "DXF" f "Y")
      (princ (strcat "\nZapisano: " f))
    )
  )
  (princ)
)

(princ "\nZaładowano TRUTOPS_CLEAN.lsp — komendy: TRUTOPS-CLEAN, TRUTOPS-EXPORT")
(princ)
