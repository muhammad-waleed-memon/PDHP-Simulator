import React, { useState } from 'react'
import { patientsApi, type PatientRecord } from '../api/client'
import { UserCheck, Shield, AlertCircle, Copy, Check, X } from 'lucide-react'

export const PatientRegisterModal: React.FC<{ isOpen: boolean; onClose: () => void }> = ({
  isOpen,
  onClose,
}) => {
  const [fullName, setFullName] = useState('')
  const [dob, setDob] = useState('1992-08-15')
  const [gender, setGender] = useState('Female')
  const [bloodGroup, setBloodGroup] = useState('B+')
  const [phone, setPhone] = useState('+92-300-5551234')
  const [city, setCity] = useState('Lahore')
  const [province, setProvince] = useState('Punjab')
  
  const [registeredPatient, setRegisteredPatient] = useState<PatientRecord | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [copied, setCopied] = useState(false)

  if (!isOpen) return null

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setError(null)
    setIsSubmitting(true)

    try {
      const result = await patientsApi.registerPatient({
        full_name: fullName,
        date_of_birth: dob,
        gender,
        blood_group: bloodGroup,
        phone_number: phone,
        city,
        province,
      })
      setRegisteredPatient(result)
    } catch (err: any) {
      const msg = err.response?.data?.error?.message || 'Patient registration failed.'
      setError(msg)
    } finally {
      setIsSubmitting(false)
    }
  }

  const copyDhid = () => {
    if (registeredPatient) {
      navigator.clipboard.writeText(registeredPatient.digital_health_id)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-md">
      <div className="relative w-full max-w-lg bg-slate-900 border border-slate-800 rounded-3xl p-6 shadow-2xl overflow-hidden">
        {/* Close Button */}
        <button
          onClick={onClose}
          className="absolute top-4 right-4 p-2 text-slate-400 hover:text-slate-200 bg-slate-800/50 hover:bg-slate-800 rounded-full transition-colors"
        >
          <X className="w-5 h-5" />
        </button>

        {registeredPatient ? (
          <div className="text-center py-4">
            <div className="w-14 h-14 bg-emerald-500/10 border border-emerald-500/30 text-emerald-400 rounded-2xl flex items-center justify-center mx-auto mb-4">
              <Shield className="w-8 h-8" />
            </div>

            <h3 className="text-xl font-bold text-slate-100">Patient Registered Successfully!</h3>
            <p className="text-xs text-slate-400 mt-1">
              A unique, opaque Digital Health ID has been generated and persisted.
            </p>

            <div className="my-6 p-4 bg-slate-950 border border-emerald-500/30 rounded-2xl relative">
              <span className="text-xs uppercase font-semibold text-emerald-400 tracking-wider block mb-1">
                Digital Health ID (DHID)
              </span>
              <div className="font-mono text-xl font-extrabold text-slate-100 flex items-center justify-center space-x-2">
                <span>{registeredPatient.digital_health_id}</span>
                <button
                  onClick={copyDhid}
                  className="p-1.5 text-slate-400 hover:text-emerald-400 hover:bg-slate-800 rounded-lg transition-colors"
                  title="Copy DHID"
                >
                  {copied ? <Check className="w-4 h-4 text-emerald-400" /> : <Copy className="w-4 h-4" />}
                </button>
              </div>
            </div>

            <div className="bg-slate-800/40 p-3 rounded-xl text-left text-xs space-y-1 text-slate-300 mb-6">
              <div><strong className="text-slate-400">Name:</strong> {registeredPatient.full_name}</div>
              <div><strong className="text-slate-400">DOB:</strong> {registeredPatient.date_of_birth} ({registeredPatient.gender})</div>
              <div><strong className="text-slate-400">Blood Group:</strong> {registeredPatient.blood_group || 'N/A'}</div>
              <div><strong className="text-slate-400">Location:</strong> {registeredPatient.city}, {registeredPatient.province}</div>
            </div>

            <button
              onClick={onClose}
              className="w-full bg-emerald-600 hover:bg-emerald-500 text-white font-medium py-2.5 rounded-xl transition-all"
            >
              Done
            </button>
          </div>
        ) : (
          <div>
            <div className="flex items-center space-x-3 mb-5">
              <div className="p-3 bg-emerald-500/10 border border-emerald-500/20 rounded-xl text-emerald-400">
                <UserCheck className="w-6 h-6" />
              </div>
              <div>
                <h3 className="text-xl font-bold text-slate-100">Patient Registration</h3>
                <p className="text-xs text-slate-400">Generate Digital Health Identity Record</p>
              </div>
            </div>

            {error && (
              <div className="mb-4 p-3 bg-rose-500/10 border border-rose-500/30 rounded-xl text-rose-400 text-xs flex items-center space-x-2">
                <AlertCircle className="w-4 h-4 flex-shrink-0" />
                <span>{error}</span>
              </div>
            )}

            <form onSubmit={handleSubmit} className="space-y-3">
              <div>
                <label className="block text-xs font-medium text-slate-300 mb-1">Full Name *</label>
                <input
                  type="text"
                  required
                  value={fullName}
                  onChange={(e) => setFullName(e.target.value)}
                  placeholder="e.g. Fatima Ali"
                  className="w-full bg-slate-950 border border-slate-800 rounded-xl px-3 py-2 text-sm text-slate-100 focus:outline-none focus:border-emerald-500"
                />
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-xs font-medium text-slate-300 mb-1">Date of Birth *</label>
                  <input
                    type="date"
                    required
                    value={dob}
                    onChange={(e) => setDob(e.target.value)}
                    className="w-full bg-slate-950 border border-slate-800 rounded-xl px-3 py-2 text-sm text-slate-100 focus:outline-none focus:border-emerald-500"
                  />
                </div>

                <div>
                  <label className="block text-xs font-medium text-slate-300 mb-1">Gender *</label>
                  <select
                    value={gender}
                    onChange={(e) => setGender(e.target.value)}
                    className="w-full bg-slate-950 border border-slate-800 rounded-xl px-3 py-2 text-sm text-slate-100 focus:outline-none focus:border-emerald-500"
                  >
                    <option value="Female">Female</option>
                    <option value="Male">Male</option>
                    <option value="Other">Other</option>
                  </select>
                </div>
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-xs font-medium text-slate-300 mb-1">Blood Group</label>
                  <select
                    value={bloodGroup}
                    onChange={(e) => setBloodGroup(e.target.value)}
                    className="w-full bg-slate-950 border border-slate-800 rounded-xl px-3 py-2 text-sm text-slate-100 focus:outline-none focus:border-emerald-500"
                  >
                    <option value="A+">A+</option>
                    <option value="A-">A-</option>
                    <option value="B+">B+</option>
                    <option value="B-">B-</option>
                    <option value="O+">O+</option>
                    <option value="O-">O-</option>
                    <option value="AB+">AB+</option>
                    <option value="AB-">AB-</option>
                  </select>
                </div>

                <div>
                  <label className="block text-xs font-medium text-slate-300 mb-1">Phone Number</label>
                  <input
                    type="text"
                    value={phone}
                    onChange={(e) => setPhone(e.target.value)}
                    placeholder="+92-300-XXXXXXX"
                    className="w-full bg-slate-950 border border-slate-800 rounded-xl px-3 py-2 text-sm text-slate-100 focus:outline-none focus:border-emerald-500"
                  />
                </div>
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-xs font-medium text-slate-300 mb-1">City</label>
                  <input
                    type="text"
                    value={city}
                    onChange={(e) => setCity(e.target.value)}
                    className="w-full bg-slate-950 border border-slate-800 rounded-xl px-3 py-2 text-sm text-slate-100 focus:outline-none focus:border-emerald-500"
                  />
                </div>

                <div>
                  <label className="block text-xs font-medium text-slate-300 mb-1">Province</label>
                  <input
                    type="text"
                    value={province}
                    onChange={(e) => setProvince(e.target.value)}
                    className="w-full bg-slate-950 border border-slate-800 rounded-xl px-3 py-2 text-sm text-slate-100 focus:outline-none focus:border-emerald-500"
                  />
                </div>
              </div>

              <button
                type="submit"
                disabled={isSubmitting}
                className="w-full mt-4 bg-gradient-to-r from-emerald-600 to-teal-600 hover:from-emerald-500 hover:to-teal-500 text-white font-medium py-2.5 rounded-xl transition-all disabled:opacity-50"
              >
                {isSubmitting ? 'Registering Patient...' : 'Create & Generate Digital Health ID'}
              </button>
            </form>
          </div>
        )}
      </div>
    </div>
  )
}
