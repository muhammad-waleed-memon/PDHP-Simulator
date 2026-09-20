import React, { useState } from 'react'
import { useAuth } from '../context/AuthContext'
import { KeyRound, Mail, LogIn, AlertCircle, ShieldCheck } from 'lucide-react'

const DEMO_PRESETS = [
  { label: 'Doctor', email: 'doctor@demo.local', pass: 'DoctorDemo123!' },
  { label: 'Patient', email: 'patient@demo.local', pass: 'PatientDemo123!' },
  { label: 'System Admin', email: 'sysadmin@demo.local', pass: 'SysAdminDemo123!' },
  { label: 'Lab User', email: 'lab@demo.local', pass: 'LabUserDemo123!' },
  { label: 'Facility Admin', email: 'facilityadmin@demo.local', pass: 'FacilityAdminDemo123!' },
]

export const LoginForm: React.FC = () => {
  const { login } = useAuth()
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [isSubmitting, setIsSubmitting] = useState(false)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setError(null)
    setIsSubmitting(true)

    try {
      await login(email, password)
    } catch (err: any) {
      const msg = err.response?.data?.error?.message || 'Authentication failed. Please check your credentials.'
      setError(msg)
    } finally {
      setIsSubmitting(false)
    }
  }

  const applyPreset = (presetEmail: string, presetPass: string) => {
    setEmail(presetEmail)
    setPassword(presetPass)
    setError(null)
  }

  return (
    <div className="w-full max-w-md mx-auto bg-slate-900/90 border border-slate-800 rounded-2xl p-6 shadow-2xl backdrop-blur-xl">
      <div className="flex items-center space-x-3 mb-6">
        <div className="p-3 bg-emerald-500/10 border border-emerald-500/20 rounded-xl text-emerald-400">
          <ShieldCheck className="w-6 h-6" />
        </div>
        <div>
          <h2 className="text-xl font-bold text-slate-100">Portal Login</h2>
          <p className="text-xs text-slate-400">Secure Argon2id + JWT Authentication</p>
        </div>
      </div>

      {/* Demo Presets Quick Selector */}
      <div className="mb-5 p-3 bg-slate-800/50 border border-slate-700/50 rounded-xl">
        <span className="text-xs font-semibold text-emerald-400 uppercase tracking-wider block mb-2">
          Demo Preset Accounts
        </span>
        <div className="flex flex-wrap gap-1.5">
          {DEMO_PRESETS.map((p) => (
            <button
              key={p.email}
              type="button"
              onClick={() => applyPreset(p.email, p.pass)}
              className="text-xs px-2.5 py-1 bg-slate-800 hover:bg-emerald-600/30 text-slate-300 hover:text-emerald-300 border border-slate-700 hover:border-emerald-500/50 rounded-lg transition-all"
            >
              {p.label}
            </button>
          ))}
        </div>
      </div>

      {error && (
        <div className="mb-4 p-3 bg-rose-500/10 border border-rose-500/30 rounded-xl text-rose-400 text-xs flex items-center space-x-2">
          <AlertCircle className="w-4 h-4 flex-shrink-0" />
          <span>{error}</span>
        </div>
      )}

      <form onSubmit={handleSubmit} className="space-y-4">
        <div>
          <label className="block text-xs font-medium text-slate-300 mb-1">Email Address</label>
          <div className="relative">
            <Mail className="w-4 h-4 text-slate-500 absolute left-3 top-3" />
            <input
              type="email"
              required
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="e.g. doctor@demo.local"
              className="w-full bg-slate-950 border border-slate-800 rounded-xl pl-9 pr-4 py-2 text-sm text-slate-100 focus:outline-none focus:border-emerald-500 transition-colors"
            />
          </div>
        </div>

        <div>
          <label className="block text-xs font-medium text-slate-300 mb-1">Password</label>
          <div className="relative">
            <KeyRound className="w-4 h-4 text-slate-500 absolute left-3 top-3" />
            <input
              type="password"
              required
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="••••••••••••"
              className="w-full bg-slate-950 border border-slate-800 rounded-xl pl-9 pr-4 py-2 text-sm text-slate-100 focus:outline-none focus:border-emerald-500 transition-colors"
            />
          </div>
        </div>

        <button
          type="submit"
          disabled={isSubmitting}
          className="w-full mt-2 bg-gradient-to-r from-emerald-600 to-teal-600 hover:from-emerald-500 hover:to-teal-500 text-white font-medium py-2.5 px-4 rounded-xl shadow-lg shadow-emerald-900/30 flex items-center justify-center space-x-2 transition-all disabled:opacity-50"
        >
          <LogIn className="w-4 h-4" />
          <span>{isSubmitting ? 'Authenticating...' : 'Sign In'}</span>
        </button>
      </form>
    </div>
  )
}
