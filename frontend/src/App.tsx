import React, { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { healthApi, patientsApi, type PatientRecord, type ReadinessResponse } from './api/client'
import { AuthProvider, useAuth } from './context/AuthContext'
import { LoginForm } from './components/LoginForm'
import { RegisterForm } from './components/RegisterForm'
import { PatientRegisterModal } from './components/PatientRegisterModal'
import { ClinicalDashboard } from './components/ClinicalDashboard'
import {
  Shield,
  Database,
  Server,
  CheckCircle2,
  UserCheck,
  Search,
  LogOut,
  User,
  BadgeAlert,
  Sparkles,
  ShieldAlert,
} from 'lucide-react'

function DashboardContent() {
  const { user, logout } = useAuth()
  const [authMode, setAuthMode] = useState<'login' | 'register'>('login')
  const [isPatientModalOpen, setIsPatientModalOpen] = useState(false)
  
  // DHID Search state
  const [searchDhid, setSearchDhid] = useState('')
  const [searchedPatient, setSearchedPatient] = useState<PatientRecord | null>(null)
  const [searchError, setSearchError] = useState<string | null>(null)
  const [isSearching, setIsSearching] = useState(false)

  // Fetch backend readiness check
  const { data: readiness, isLoading: isReadinessLoading } = useQuery<ReadinessResponse>({
    queryKey: ['health-readiness'],
    queryFn: healthApi.readiness,
    refetchInterval: 10000,
  })

  const isDbHealthy = readiness?.checks?.database?.status === 'ok'

  const handleDhidSearch = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!searchDhid.trim()) return
    setSearchError(null)
    setSearchedPatient(null)
    setIsSearching(true)

    try {
      const res = await patientsApi.getByDigitalHealthId(searchDhid.trim())
      setSearchedPatient(res)
    } catch (err: any) {
      const msg = err.response?.data?.error?.message || 'Patient lookup failed. Verify Digital Health ID.'
      setSearchError(msg)
    } finally {
      setIsSearching(false)
    }
  }

  return (
    <div className="min-w-[320px] min-h-screen bg-slate-950 text-slate-100 font-sans selection:bg-emerald-500 selection:text-white">
      {/* Background Decorator Gradients */}
      <div className="fixed inset-0 pointer-events-none overflow-hidden">
        <div className="absolute -top-40 -right-40 w-96 h-96 bg-emerald-600/10 rounded-full blur-3xl" />
        <div className="absolute top-1/3 -left-40 w-96 h-96 bg-teal-600/10 rounded-full blur-3xl" />
      </div>

      {/* Top Navbar */}
      <header className="sticky top-0 z-40 bg-slate-950/80 backdrop-blur-xl border-b border-slate-800/80">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 h-16 flex items-center justify-between">
          <div className="flex items-center space-x-3">
            <div className="w-10 h-10 bg-gradient-to-br from-emerald-500 to-teal-600 rounded-xl flex items-center justify-center shadow-lg shadow-emerald-900/30">
              <Shield className="w-6 h-6 text-white" />
            </div>
            <div>
              <h1 className="text-base font-bold text-slate-100 tracking-tight leading-none">
                Pakistan Digital Health Platform
              </h1>
              <p className="text-[11px] text-slate-400 font-medium">Digital Identity & Medical Records</p>
            </div>
          </div>

          <div className="flex items-center space-x-4">
            {/* Health Badge */}
            <div className="hidden sm:flex items-center space-x-2 px-3 py-1.5 bg-slate-900/90 border border-slate-800 rounded-full text-xs">
              <span className="relative flex h-2 w-2">
                <span className={`animate-ping absolute inline-flex h-full w-full rounded-full opacity-75 ${isDbHealthy ? 'bg-emerald-400' : 'bg-rose-400'}`} />
                <span className={`relative inline-flex rounded-full h-2 w-2 ${isDbHealthy ? 'bg-emerald-500' : 'bg-rose-500'}`} />
              </span>
              <span className="text-slate-300 font-medium">
                {isReadinessLoading ? 'Connecting...' : isDbHealthy ? 'System Active' : 'Degraded'}
              </span>
            </div>

            {user ? (
              <div className="flex items-center space-x-3">
                <div className="text-right hidden md:block">
                  <div className="text-xs font-semibold text-slate-200">{user.full_name}</div>
                  <div className="text-[10px] text-emerald-400 font-mono tracking-wider">{user.role}</div>
                </div>
                <button
                  onClick={logout}
                  className="p-2 text-slate-400 hover:text-rose-400 bg-slate-900 hover:bg-slate-800 border border-slate-800 rounded-xl transition-colors"
                  title="Sign Out"
                >
                  <LogOut className="w-4 h-4" />
                </button>
              </div>
            ) : (
              <div className="flex bg-slate-900 border border-slate-800 rounded-xl p-1 text-xs font-medium">
                <button
                  onClick={() => setAuthMode('login')}
                  className={`px-3 py-1 rounded-lg transition-all ${authMode === 'login' ? 'bg-emerald-600 text-white shadow' : 'text-slate-400 hover:text-slate-200'}`}
                >
                  Sign In
                </button>
                <button
                  onClick={() => setAuthMode('register')}
                  className={`px-3 py-1 rounded-lg transition-all ${authMode === 'register' ? 'bg-emerald-600 text-white shadow' : 'text-slate-400 hover:text-slate-200'}`}
                >
                  Register
                </button>
              </div>
            )}
          </div>
        </div>
      </header>

      {/* Main Container */}
      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8 space-y-8">
        {/* Phase Header Banner */}
        <div className="bg-gradient-to-r from-slate-900 via-slate-900 to-slate-950 border border-slate-800/80 rounded-3xl p-6 sm:p-8 shadow-2xl relative overflow-hidden">
          <div className="absolute top-0 right-0 w-80 h-80 bg-emerald-500/5 rounded-full blur-3xl pointer-events-none" />
          <div className="relative z-10 max-w-3xl">
            <div className="inline-flex items-center space-x-2 px-3 py-1 bg-emerald-500/10 border border-emerald-500/20 text-emerald-400 rounded-full text-xs font-semibold mb-4">
              <Sparkles className="w-3.5 h-3.5" />
              <span>Phase 2 — Identity & Authentication Verification</span>
            </div>
            <h2 className="text-2xl sm:text-3xl font-extrabold text-slate-100 tracking-tight">
              Digital Health Identity & Argon2 / JWT Security Layer
            </h2>
            <p className="mt-2 text-sm text-slate-400 leading-relaxed">
              Real functional implementation of opaque Digital Health ID generation (`PK-HID-XXXX-XXXX-XXXX`), Argon2id password security, role-based identity claims, and PostgreSQL database identity persistence.
            </p>
          </div>
        </div>

        {/* Content Section: Auth Form vs User Dashboard */}
        {!user ? (
          <div className="grid grid-cols-1 lg:grid-cols-12 gap-8 items-start">
            <div className="lg:col-span-5">
              {authMode === 'login' ? <LoginForm /> : <RegisterForm onSuccess={() => setAuthMode('login')} />}
            </div>

            <div className="lg:col-span-7 space-y-6">
              {/* Architecture & Verification Info */}
              <div className="bg-slate-900/60 border border-slate-800 rounded-2xl p-6 space-y-4">
                <h3 className="text-base font-bold text-slate-100 flex items-center space-x-2">
                  <ShieldAlert className="w-5 h-5 text-emerald-400" />
                  <span>Phase 2 Technical Foundations</span>
                </h3>

                <ul className="space-y-3 text-xs text-slate-300">
                  <li className="flex items-start space-x-2.5">
                    <CheckCircle2 className="w-4 h-4 text-emerald-400 flex-shrink-0 mt-0.5" />
                    <span><strong>Opaque Digital Health IDs:</strong> Structured `PK-HID-XXXX-XXXX-XXXX` format, non-sequential and completely detached from CNIC numbers.</span>
                  </li>
                  <li className="flex items-start space-x-2.5">
                    <CheckCircle2 className="w-4 h-4 text-emerald-400 flex-shrink-0 mt-0.5" />
                    <span><strong>Argon2id Password Security:</strong> Standard PHC password hashing with custom salt. Zero plaintext passwords stored or logged.</span>
                  </li>
                  <li className="flex items-start space-x-2.5">
                    <CheckCircle2 className="w-4 h-4 text-emerald-400 flex-shrink-0 mt-0.5" />
                    <span><strong>JWT Stateless Authentication:</strong> Secure claims with `sub`, `email`, `role`, and configurable expiration.</span>
                  </li>
                  <li className="flex items-start space-x-2.5">
                    <CheckCircle2 className="w-4 h-4 text-emerald-400 flex-shrink-0 mt-0.5" />
                    <span><strong>Transactional Integrity:</strong> Patient & Doctor creation uses single SQLx database transactions to prevent orphaned records.</span>
                  </li>
                </ul>

                <div className="pt-2 text-[11px] text-slate-500 italic border-t border-slate-800">
                  Note: Demographics and identities are simulated prototype records. National verification is not connected to external government databases.
                </div>
              </div>
            </div>
          </div>
        ) : (
          /* Authenticated User View */
          <div className="space-y-8">
            {/* Logged in User Bar */}
            <div className="bg-slate-900 border border-emerald-500/30 rounded-2xl p-6 flex flex-wrap items-center justify-between gap-4">
              <div className="flex items-center space-x-4">
                <div className="w-12 h-12 bg-emerald-500/10 border border-emerald-500/20 text-emerald-400 rounded-2xl flex items-center justify-center">
                  <User className="w-6 h-6" />
                </div>
                <div>
                  <div className="text-xs text-slate-400 uppercase font-semibold tracking-wider">Authenticated Identity</div>
                  <div className="text-lg font-bold text-slate-100">{user.full_name}</div>
                  <div className="text-xs text-slate-400">{user.email}</div>
                </div>
              </div>

              <div className="flex items-center space-x-3">
                <div className="px-3 py-1.5 bg-emerald-500/10 border border-emerald-500/30 text-emerald-300 rounded-xl text-xs font-mono font-bold">
                  ROLE: {user.role}
                </div>

                <button
                  onClick={() => setIsPatientModalOpen(true)}
                  className="bg-gradient-to-r from-emerald-600 to-teal-600 hover:from-emerald-500 hover:to-teal-500 text-white text-xs font-semibold py-2 px-4 rounded-xl shadow-lg shadow-emerald-900/30 flex items-center space-x-2 transition-all"
                >
                  <UserCheck className="w-4 h-4" />
                  <span>Register Patient & Generate DHID</span>
                </button>
              </div>
            </div>

            {/* Digital Health ID Lookup Tool */}
            <div className="bg-slate-900/80 border border-slate-800 rounded-2xl p-6 space-y-4">
              <div className="flex items-center space-x-3">
                <div className="p-2.5 bg-teal-500/10 border border-teal-500/20 rounded-xl text-teal-400">
                  <Search className="w-5 h-5" />
                </div>
                <div>
                  <h3 className="text-base font-bold text-slate-100">Digital Health ID Patient Lookup</h3>
                  <p className="text-xs text-slate-400">Test patient record query by opaque Digital Health ID (`GET /api/v1/patients/by-digital-health-id/{'{dhid}'}`)</p>
                </div>
              </div>

              <form onSubmit={handleDhidSearch} className="flex flex-col sm:flex-row gap-3">
                <input
                  type="text"
                  value={searchDhid}
                  onChange={(e) => setSearchDhid(e.target.value)}
                  placeholder="Enter Digital Health ID (e.g. PK-HID-8F3A-4B2C-9E10)"
                  className="flex-1 bg-slate-950 border border-slate-800 rounded-xl px-4 py-2.5 text-sm font-mono text-slate-100 focus:outline-none focus:border-teal-500"
                />
                <button
                  type="submit"
                  disabled={isSearching || !searchDhid.trim()}
                  className="bg-teal-600 hover:bg-teal-500 text-white text-xs font-semibold px-6 py-2.5 rounded-xl transition-all disabled:opacity-50 flex items-center justify-center space-x-2"
                >
                  <Search className="w-4 h-4" />
                  <span>{isSearching ? 'Querying...' : 'Lookup Patient'}</span>
                </button>
              </form>

              {searchError && (
                <div className="p-3 bg-rose-500/10 border border-rose-500/30 rounded-xl text-rose-400 text-xs flex items-center space-x-2">
                  <BadgeAlert className="w-4 h-4 flex-shrink-0" />
                  <span>{searchError}</span>
                </div>
              )}

              {searchedPatient && (
                <div className="space-y-6">
                  <div className="p-4 bg-slate-950 border border-emerald-500/30 rounded-xl space-y-2 text-xs">
                    <div className="flex items-center justify-between border-b border-slate-800 pb-2">
                      <span className="font-bold text-slate-100 text-sm">{searchedPatient.full_name}</span>
                      <span className="font-mono text-emerald-400 font-bold">{searchedPatient.digital_health_id}</span>
                    </div>
                    <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 text-slate-300">
                      <div><strong className="text-slate-500">DOB:</strong> {searchedPatient.date_of_birth}</div>
                      <div><strong className="text-slate-500">Gender:</strong> {searchedPatient.gender}</div>
                      <div><strong className="text-slate-500">Blood:</strong> {searchedPatient.blood_group || 'N/A'}</div>
                      <div><strong className="text-slate-500">Status:</strong> <span className="text-emerald-400 font-semibold">{searchedPatient.status}</span></div>
                    </div>
                  </div>

                  <ClinicalDashboard patient={searchedPatient} />
                </div>
              )}
            </div>
          </div>
        )}

        {/* System Component Status */}
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
          <div className="bg-slate-900/60 border border-slate-800 rounded-2xl p-4 flex items-center space-x-3">
            <Server className="w-6 h-6 text-emerald-400" />
            <div>
              <div className="text-xs text-slate-400">Rust Axum Backend</div>
              <div className="text-xs font-bold text-slate-200">REST API v1</div>
            </div>
          </div>

          <div className="bg-slate-900/60 border border-slate-800 rounded-2xl p-4 flex items-center space-x-3">
            <Database className="w-6 h-6 text-teal-400" />
            <div>
              <div className="text-xs text-slate-400">PostgreSQL Database</div>
              <div className="text-xs font-bold text-slate-200">
                {isDbHealthy ? 'Connected (Pool Active)' : 'Pending Connection'}
              </div>
            </div>
          </div>

          <div className="bg-slate-900/60 border border-slate-800 rounded-2xl p-4 flex items-center space-x-3">
            <Shield className="w-6 h-6 text-emerald-400" />
            <div>
              <div className="text-xs text-slate-400">Security Architecture</div>
              <div className="text-xs font-bold text-slate-200">Argon2id + JWT Active</div>
            </div>
          </div>
        </div>

        {/* Modal for Patient Registration */}
        <PatientRegisterModal
          isOpen={isPatientModalOpen}
          onClose={() => setIsPatientModalOpen(false)}
        />
      </main>
    </div>
  )
}

export default function App() {
  return (
    <AuthProvider>
      <DashboardContent />
    </AuthProvider>
  )
}
