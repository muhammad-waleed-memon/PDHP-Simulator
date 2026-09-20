import React, { useState } from 'react'
import { authApi, normalizeRole, type UserRole } from '../api/client'
import { useAuth } from '../context/AuthContext'
import { UserPlus, Mail, KeyRound, User, AlertCircle, CheckCircle2 } from 'lucide-react'

export const RegisterForm: React.FC<{ onSuccess?: () => void }> = ({ onSuccess }) => {
  const { login } = useAuth()
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [fullName, setFullName] = useState('')
  const [role, setRole] = useState<UserRole>('Patient')
  const [error, setError] = useState<string | null>(null)
  const [isSubmitting, setIsSubmitting] = useState(false)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setError(null)
    setIsSubmitting(true)

    try {
      await authApi.register({
        email,
        password,
        full_name: fullName,
        role,
      })
      // Auto-login after registration
      await login(email, password)
      if (onSuccess) onSuccess()
    } catch (err: any) {
      const msg = err.response?.data?.error?.message || 'Registration failed. Please try again.'
      setError(msg)
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
    <div className="w-full max-w-md mx-auto bg-slate-900/90 border border-slate-800 rounded-2xl p-6 shadow-2xl backdrop-blur-xl">
      <div className="flex items-center space-x-3 mb-6">
        <div className="p-3 bg-teal-500/10 border border-teal-500/20 rounded-xl text-teal-400">
          <UserPlus className="w-6 h-6" />
        </div>
        <div>
          <h2 className="text-xl font-bold text-slate-100">Create Account</h2>
          <p className="text-xs text-slate-400">Register new identity credentials</p>
        </div>
      </div>

      {error && (
        <div className="mb-4 p-3 bg-rose-500/10 border border-rose-500/30 rounded-xl text-rose-400 text-xs flex items-center space-x-2">
          <AlertCircle className="w-4 h-4 flex-shrink-0" />
          <span>{error}</span>
        </div>
      )}

      <form onSubmit={handleSubmit} className="space-y-3.5">
        <div>
          <label className="block text-xs font-medium text-slate-300 mb-1">Full Name</label>
          <div className="relative">
            <User className="w-4 h-4 text-slate-500 absolute left-3 top-3" />
            <input
              type="text"
              required
              value={fullName}
              onChange={(e) => setFullName(e.target.value)}
              placeholder="e.g. Dr. Ayesha Khan"
              className="w-full bg-slate-950 border border-slate-800 rounded-xl pl-9 pr-4 py-2 text-sm text-slate-100 focus:outline-none focus:border-teal-500 transition-colors"
            />
          </div>
        </div>

        <div>
          <label className="block text-xs font-medium text-slate-300 mb-1">Email Address</label>
          <div className="relative">
            <Mail className="w-4 h-4 text-slate-500 absolute left-3 top-3" />
            <input
              type="email"
              required
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="user@example.com"
              className="w-full bg-slate-950 border border-slate-800 rounded-xl pl-9 pr-4 py-2 text-sm text-slate-100 focus:outline-none focus:border-teal-500 transition-colors"
            />
          </div>
        </div>

        <div>
          <label className="block text-xs font-medium text-slate-300 mb-1">Password (min 8 chars)</label>
          <div className="relative">
            <KeyRound className="w-4 h-4 text-slate-500 absolute left-3 top-3" />
            <input
              type="password"
              required
              minLength={8}
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="••••••••••••"
              className="w-full bg-slate-950 border border-slate-800 rounded-xl pl-9 pr-4 py-2 text-sm text-slate-100 focus:outline-none focus:border-teal-500 transition-colors"
            />
          </div>
        </div>

        <div>
          <label className="block text-xs font-medium text-slate-300 mb-1">Account Role</label>
          <select
            value={role}
            onChange={(e) => setRole(normalizeRole(e.target.value))}
            className="w-full bg-slate-950 border border-slate-800 rounded-xl px-3 py-2 text-sm text-slate-100 focus:outline-none focus:border-teal-500 transition-colors"
          >
            <option value="Patient">Patient (PATIENT)</option>
            <option value="Doctor">Doctor / Provider (DOCTOR)</option>
            <option value="Lab">Lab Technician (LAB)</option>
            <option value="FacilityAdmin">Facility Administrator (FACILITY_ADMIN)</option>
            <option value="SystemAdmin">System Administrator (SYSTEM_ADMIN)</option>
          </select>
        </div>

        <button
          type="submit"
          disabled={isSubmitting}
          className="w-full mt-3 bg-gradient-to-r from-teal-600 to-emerald-600 hover:from-teal-500 hover:to-emerald-500 text-white font-medium py-2.5 px-4 rounded-xl shadow-lg shadow-teal-900/30 flex items-center justify-center space-x-2 transition-all disabled:opacity-50"
        >
          <CheckCircle2 className="w-4 h-4" />
          <span>{isSubmitting ? 'Creating Account...' : 'Register Account'}</span>
        </button>
      </form>
    </div>
  )
}
