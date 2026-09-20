import React, { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import {
  clinicalApi,
  fhirApi,
  type PatientRecord,
  type Condition,
  type Allergy,
  type PrescriptionWithDetails,
  type LabReport,
  type TimelineItem,
  type Medication,
} from '../api/client'
import { useAuth } from '../context/AuthContext'
import {
  Activity,
  AlertTriangle,
  Pill,
  Clock,
  Plus,
  ShieldAlert,
  FlaskConical,
  X,
  AlertOctagon,
  FileCode,
  Copy,
  Check,
} from 'lucide-react'

interface Props {
  patient: PatientRecord
}

export function ClinicalDashboard({ patient }: Props) {
  const { user } = useAuth()
  const queryClient = useQueryClient()
  const [activeTab, setActiveTab] = useState<'timeline' | 'conditions' | 'allergies' | 'prescriptions' | 'labs' | 'fhir'>('timeline')
  const [fhirViewMode, setFhirViewMode] = useState<'everything' | 'patient'>('everything')
  const [copied, setCopied] = useState(false)

  // Modals state
  const [isConditionModalOpen, setIsConditionModalOpen] = useState(false)
  const [isAllergyModalOpen, setIsAllergyModalOpen] = useState(false)
  const [isPrescriptionModalOpen, setIsPrescriptionModalOpen] = useState(false)
  const [isLabModalOpen, setIsLabModalOpen] = useState(false)

  // Safety alert modal state for prescriptions
  const [safetyAlertMsg, setSafetyAlertMsg] = useState<string | null>(null)
  const [overrideReason, setOverrideReason] = useState('')

  // Form states
  const [conditionName, setConditionName] = useState('')
  const [conditionNotes, setConditionNotes] = useState('')

  const [allergen, setAllergen] = useState('')
  const [allergySeverity, setAllergySeverity] = useState('MODERATE')
  const [allergyReaction, setAllergyReaction] = useState('')

  const [selectedMedId, setSelectedMedId] = useState('')
  const [dosage, setDosage] = useState('500mg')
  const [frequency, setFrequency] = useState('Twice daily (BID)')
  const [duration, setDuration] = useState('7 days')
  const [rxNotes, setRxNotes] = useState('')

  const [testName, setTestName] = useState('')
  const [result, setResult] = useState('')
  const [unit, setUnit] = useState('mg/dL')
  const [isAbnormal, setIsAbnormal] = useState(false)

  // Queries
  const { data: timeline } = useQuery<TimelineItem[]>({
    queryKey: ['timeline', patient.id],
    queryFn: () => clinicalApi.getTimeline(patient.id),
  })

  const { data: conditions } = useQuery<Condition[]>({
    queryKey: ['conditions', patient.id],
    queryFn: () => clinicalApi.getConditions(patient.id),
  })

  const { data: allergies } = useQuery<Allergy[]>({
    queryKey: ['allergies', patient.id],
    queryFn: () => clinicalApi.getAllergies(patient.id),
  })

  const { data: prescriptions } = useQuery<PrescriptionWithDetails[]>({
    queryKey: ['prescriptions', patient.id],
    queryFn: () => clinicalApi.getPrescriptions(patient.id),
  })

  const { data: labReports } = useQuery<LabReport[]>({
    queryKey: ['labs', patient.id],
    queryFn: () => clinicalApi.getLabReports(patient.id),
  })

  const { data: medications } = useQuery<Medication[]>({
    queryKey: ['medications'],
    queryFn: () => clinicalApi.getMedications(),
  })

  // FHIR Queries
  const { data: fhirBundle, isLoading: isFhirBundleLoading } = useQuery({
    queryKey: ['fhirEverything', patient.id],
    queryFn: () => fhirApi.getPatientEverything(patient.id),
    enabled: activeTab === 'fhir' && fhirViewMode === 'everything',
  })

  const { data: fhirPatient, isLoading: isFhirPatientLoading } = useQuery({
    queryKey: ['fhirPatient', patient.id],
    queryFn: () => fhirApi.getPatient(patient.id),
    enabled: activeTab === 'fhir' && fhirViewMode === 'patient',
  })

  // Add Condition Mutation
  const addConditionMutation = useMutation({
    mutationFn: (data: any) => clinicalApi.createCondition(patient.id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['conditions', patient.id] })
      queryClient.invalidateQueries({ queryKey: ['timeline', patient.id] })
      setIsConditionModalOpen(false)
      setConditionName('')
      setConditionNotes('')
    },
  })

  // Add Allergy Mutation
  const addAllergyMutation = useMutation({
    mutationFn: (data: any) => clinicalApi.createAllergy(patient.id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['allergies', patient.id] })
      queryClient.invalidateQueries({ queryKey: ['timeline', patient.id] })
      setIsAllergyModalOpen(false)
      setAllergen('')
      setAllergyReaction('')
    },
  })

  // Create Prescription Mutation
  const createPrescriptionMutation = useMutation({
    mutationFn: (data: any) => clinicalApi.createPrescription(patient.id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['prescriptions', patient.id] })
      queryClient.invalidateQueries({ queryKey: ['timeline', patient.id] })
      setIsPrescriptionModalOpen(false)
      setSafetyAlertMsg(null)
      setOverrideReason('')
      setRxNotes('')
    },
    onError: (err: any) => {
      const msg = err.response?.data?.error?.message || 'Failed to issue prescription'
      if (msg.includes('SAFETY ALERT')) {
        setSafetyAlertMsg(msg)
      } else {
        alert(msg)
      }
    },
  })

  // Create Lab Report Mutation
  const createLabMutation = useMutation({
    mutationFn: (data: any) => clinicalApi.createLabReport(patient.id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['labs', patient.id] })
      queryClient.invalidateQueries({ queryKey: ['timeline', patient.id] })
      setIsLabModalOpen(false)
      setTestName('')
      setResult('')
    },
  })

  const handleConditionSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (!conditionName.trim()) return
    addConditionMutation.mutate({
      name: conditionName.trim(),
      notes: conditionNotes.trim() || undefined,
    })
  }

  const handleAllergySubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (!allergen.trim()) return
    addAllergyMutation.mutate({
      allergen: allergen.trim(),
      severity: allergySeverity,
      reaction: allergyReaction.trim() || undefined,
    })
  }

  const handlePrescriptionSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (!selectedMedId) {
      alert('Please select a medication from the catalog.')
      return
    }

    const payload: any = {
      clinical_notes: rxNotes,
      items: [
        {
          medication_id: selectedMedId,
          dosage,
          frequency,
          duration,
          quantity: 1,
        },
      ],
    }

    if (overrideReason.trim()) {
      payload.override_reason = overrideReason.trim()
    }

    createPrescriptionMutation.mutate(payload)
  }

  const handleLabSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (!testName.trim() || !result.trim()) return
    createLabMutation.mutate({
      test_name: testName.trim(),
      result: result.trim(),
      unit: unit.trim() || undefined,
      abnormal_flag: isAbnormal,
    })
  }

  const isDoctor = user?.role === 'Doctor' || user?.role === 'SystemAdmin'

  return (
    <div className="bg-slate-900/90 border border-slate-800 rounded-2xl p-6 shadow-xl space-y-6">
      {/* Patient Header Summary */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 pb-6 border-b border-slate-800">
        <div>
          <div className="flex items-center space-x-3">
            <h2 className="text-xl font-bold text-slate-100">{patient.full_name}</h2>
            <span className="px-2.5 py-0.5 rounded-full text-xs font-mono bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
              {patient.digital_health_id}
            </span>
          </div>
          <p className="text-xs text-slate-400 mt-1">
            DOB: {patient.date_of_birth} | Gender: {patient.gender} | Blood: {patient.blood_group || 'N/A'}
          </p>
        </div>

        <div className="flex items-center space-x-2">
          {isDoctor && (
            <button
              onClick={() => setIsPrescriptionModalOpen(true)}
              className="flex items-center space-x-2 px-3 py-2 bg-emerald-600 hover:bg-emerald-500 text-white rounded-lg text-xs font-semibold shadow-lg shadow-emerald-900/30 transition-all"
            >
              <Pill className="w-4 h-4" />
              <span>Issue Prescription</span>
            </button>
          )}
        </div>
      </div>

      {/* Tabs */}
      <div className="flex space-x-2 border-b border-slate-800 pb-2 overflow-x-auto">
        <button
          onClick={() => setActiveTab('timeline')}
          className={`flex items-center space-x-2 px-4 py-2 rounded-lg text-xs font-medium transition-all ${
            activeTab === 'timeline' ? 'bg-emerald-500/20 text-emerald-400 border border-emerald-500/30' : 'text-slate-400 hover:text-slate-200'
          }`}
        >
          <Clock className="w-4 h-4" />
          <span>Longitudinal Timeline ({timeline?.length || 0})</span>
        </button>

        <button
          onClick={() => setActiveTab('conditions')}
          className={`flex items-center space-x-2 px-4 py-2 rounded-lg text-xs font-medium transition-all ${
            activeTab === 'conditions' ? 'bg-emerald-500/20 text-emerald-400 border border-emerald-500/30' : 'text-slate-400 hover:text-slate-200'
          }`}
        >
          <Activity className="w-4 h-4" />
          <span>Conditions ({conditions?.length || 0})</span>
        </button>

        <button
          onClick={() => setActiveTab('allergies')}
          className={`flex items-center space-x-2 px-4 py-2 rounded-lg text-xs font-medium transition-all ${
            activeTab === 'allergies' ? 'bg-rose-500/20 text-rose-400 border border-rose-500/30' : 'text-slate-400 hover:text-slate-200'
          }`}
        >
          <AlertTriangle className="w-4 h-4" />
          <span>Allergies ({allergies?.length || 0})</span>
        </button>

        <button
          onClick={() => setActiveTab('prescriptions')}
          className={`flex items-center space-x-2 px-4 py-2 rounded-lg text-xs font-medium transition-all ${
            activeTab === 'prescriptions' ? 'bg-teal-500/20 text-teal-400 border border-teal-500/30' : 'text-slate-400 hover:text-slate-200'
          }`}
        >
          <Pill className="w-4 h-4" />
          <span>Prescriptions ({prescriptions?.length || 0})</span>
        </button>

        <button
          onClick={() => setActiveTab('labs')}
          className={`flex items-center space-x-2 px-4 py-2 rounded-lg text-xs font-medium transition-all ${
            activeTab === 'labs' ? 'bg-cyan-500/20 text-cyan-400 border border-cyan-500/30' : 'text-slate-400 hover:text-slate-200'
          }`}
        >
          <FlaskConical className="w-4 h-4" />
          <span>Lab Reports ({labReports?.length || 0})</span>
        </button>

        <button
          onClick={() => setActiveTab('fhir')}
          className={`flex items-center space-x-2 px-4 py-2 rounded-lg text-xs font-medium transition-all ${
            activeTab === 'fhir' ? 'bg-indigo-500/20 text-indigo-400 border border-indigo-500/30' : 'text-slate-400 hover:text-slate-200'
          }`}
        >
          <FileCode className="w-4 h-4" />
          <span>FHIR R4 View</span>
        </button>
      </div>

      {/* Tab Content */}
      {activeTab === 'timeline' && (
        <div className="space-y-4">
          <h3 className="text-sm font-semibold text-slate-300">Longitudinal Medical History</h3>
          {!timeline || timeline.length === 0 ? (
            <p className="text-xs text-slate-500 py-6 text-center">No timeline events recorded yet.</p>
          ) : (
            <div className="relative border-l-2 border-slate-800 ml-4 space-y-6 pl-6 py-2">
              {timeline.map((item) => (
                <div key={item.id} className="relative group">
                  <div className="absolute -left-[31px] top-1.5 w-3.5 h-3.5 rounded-full bg-slate-900 border-2 border-emerald-500 group-hover:scale-125 transition-all" />
                  <div className="bg-slate-950/60 border border-slate-800/80 rounded-xl p-4 space-y-1">
                    <div className="flex items-center justify-between">
                      <span className="text-xs font-bold text-slate-200">{item.title}</span>
                      <span className="text-[10px] text-slate-500 font-mono">
                        {new Date(item.timestamp).toLocaleString()}
                      </span>
                    </div>
                    <p className="text-xs text-slate-400">{item.description}</p>
                    <span className="inline-block text-[10px] font-semibold text-emerald-400 uppercase tracking-wider">
                      {item.event_type} • Status: {item.status}
                    </span>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {activeTab === 'conditions' && (
        <div className="space-y-4">
          <div className="flex justify-between items-center">
            <h3 className="text-sm font-semibold text-slate-300">Medical Conditions</h3>
            {isDoctor && (
              <button
                onClick={() => setIsConditionModalOpen(true)}
                className="flex items-center space-x-1 px-2.5 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-200 rounded-lg text-xs"
              >
                <Plus className="w-3.5 h-3.5" />
                <span>Add Condition</span>
              </button>
            )}
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {conditions?.map((c) => (
              <div key={c.id} className="bg-slate-950/80 border border-slate-800 rounded-xl p-4 space-y-2">
                <div className="flex justify-between items-start">
                  <h4 className="text-sm font-semibold text-slate-100">{c.name}</h4>
                  <span className="px-2 py-0.5 rounded text-[10px] font-semibold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                    {c.status}
                  </span>
                </div>
                {c.notes && <p className="text-xs text-slate-400">{c.notes}</p>}
                <div className="text-[10px] text-slate-500 flex justify-between pt-2 border-t border-slate-900">
                  <span>Version {c.version}</span>
                  <span>Recorded: {new Date(c.created_at).toLocaleDateString()}</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {activeTab === 'allergies' && (
        <div className="space-y-4">
          <div className="flex justify-between items-center">
            <h3 className="text-sm font-semibold text-rose-300">Patient Allergies</h3>
            {isDoctor && (
              <button
                onClick={() => setIsAllergyModalOpen(true)}
                className="flex items-center space-x-1 px-2.5 py-1.5 bg-rose-950/60 hover:bg-rose-900/60 text-rose-200 border border-rose-800/80 rounded-lg text-xs"
              >
                <Plus className="w-3.5 h-3.5" />
                <span>Record Allergy</span>
              </button>
            )}
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {allergies?.map((a) => (
              <div key={a.id} className="bg-slate-950/80 border border-rose-900/30 rounded-xl p-4 space-y-2">
                <div className="flex justify-between items-start">
                  <h4 className="text-sm font-bold text-rose-200">{a.allergen}</h4>
                  <span className="px-2 py-0.5 rounded text-[10px] font-bold bg-rose-500/20 text-rose-300 border border-rose-500/30">
                    {a.severity}
                  </span>
                </div>
                {a.reaction && <p className="text-xs text-slate-300">Reaction: {a.reaction}</p>}
                <div className="text-[10px] text-slate-500 flex justify-between pt-2 border-t border-slate-900">
                  <span>Version {a.version}</span>
                  <span>Status: {a.status}</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {activeTab === 'prescriptions' && (
        <div className="space-y-4">
          <h3 className="text-sm font-semibold text-teal-300">Issued Prescriptions</h3>
          <div className="space-y-3">
            {prescriptions?.map((rx) => (
              <div key={rx.prescription.id} className="bg-slate-950/80 border border-slate-800 rounded-xl p-4 space-y-3">
                <div className="flex justify-between items-center pb-2 border-b border-slate-900">
                  <span className="text-xs font-semibold text-slate-300">
                    Issued: {new Date(rx.prescription.issued_at).toLocaleString()}
                  </span>
                  <span className="text-xs font-mono text-teal-400 bg-teal-500/10 px-2 py-0.5 rounded">
                    {rx.prescription.status}
                  </span>
                </div>

                <div className="space-y-2">
                  {rx.items.map((it) => (
                    <div key={it.item.id} className="flex justify-between items-center text-xs text-slate-200">
                      <span className="font-semibold text-emerald-400">{it.medication_name}</span>
                      <span>{it.item.dosage} • {it.item.frequency} • {it.item.duration}</span>
                    </div>
                  ))}
                </div>

                {rx.prescription.override_reason && (
                  <div className="p-2 bg-amber-500/10 border border-amber-500/20 rounded text-[11px] text-amber-300 flex items-start space-x-1.5">
                    <ShieldAlert className="w-4 h-4 shrink-0 mt-0.5" />
                    <span>Override Reason: {rx.prescription.override_reason}</span>
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {activeTab === 'labs' && (
        <div className="space-y-4">
          <div className="flex justify-between items-center">
            <h3 className="text-sm font-semibold text-cyan-300">Laboratory Reports</h3>
            {isDoctor && (
              <button
                onClick={() => setIsLabModalOpen(true)}
                className="flex items-center space-x-1 px-2.5 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-200 rounded-lg text-xs"
              >
                <Plus className="w-3.5 h-3.5" />
                <span>Add Lab Report</span>
              </button>
            )}
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {labReports?.map((l) => (
              <div key={l.id} className="bg-slate-950/80 border border-slate-800 rounded-xl p-4 space-y-2">
                <div className="flex justify-between items-start">
                  <h4 className="text-sm font-bold text-slate-200">{l.test_name}</h4>
                  {l.abnormal_flag && (
                    <span className="px-2 py-0.5 rounded text-[10px] font-bold bg-rose-500/20 text-rose-400 border border-rose-500/30">
                      ABNORMAL
                    </span>
                  )}
                </div>
                <p className="text-base font-semibold text-emerald-400">
                  {l.result} {l.unit}
                </p>
                <span className="text-[10px] text-slate-500">Date: {new Date(l.report_date).toLocaleDateString()}</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {activeTab === 'fhir' && (
        <div className="space-y-4">
          <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-3">
            <div>
              <h3 className="text-sm font-semibold text-indigo-300">FHIR R4 Interoperability View</h3>
              <p className="text-xs text-slate-400">
                Exposes clinical domain entities formatted as standards-compliant HL7 FHIR R4 JSON payloads.
              </p>
            </div>
            <div className="flex items-center space-x-2">
              <button
                onClick={() => setFhirViewMode('everything')}
                className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-all ${
                  fhirViewMode === 'everything' ? 'bg-indigo-600 text-white' : 'bg-slate-800 text-slate-300 hover:bg-slate-700'
                }`}
              >
                Bundle ($everything)
              </button>
              <button
                onClick={() => setFhirViewMode('patient')}
                className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-all ${
                  fhirViewMode === 'patient' ? 'bg-indigo-600 text-white' : 'bg-slate-800 text-slate-300 hover:bg-slate-700'
                }`}
              >
                Patient Resource
              </button>
              <button
                onClick={() => {
                  const dataToCopy = fhirViewMode === 'everything' ? fhirBundle : fhirPatient
                  if (dataToCopy) {
                    navigator.clipboard.writeText(JSON.stringify(dataToCopy, null, 2))
                    setCopied(true)
                    setTimeout(() => setCopied(false), 2000)
                  }
                }}
                className="flex items-center space-x-1 px-3 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-200 rounded-lg text-xs font-medium"
              >
                {copied ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
                <span>{copied ? 'Copied!' : 'Copy JSON'}</span>
              </button>
            </div>
          </div>

          <div className="bg-slate-950 border border-slate-800 rounded-xl p-4 overflow-x-auto max-h-[500px]">
            {(isFhirBundleLoading || isFhirPatientLoading) ? (
              <p className="text-xs text-slate-400 animate-pulse">Loading FHIR R4 payload...</p>
            ) : (
              <pre className="text-xs font-mono text-emerald-400 whitespace-pre-wrap">
                {JSON.stringify(fhirViewMode === 'everything' ? fhirBundle : fhirPatient, null, 2)}
              </pre>
            )}
          </div>
        </div>
      )}

      {/* Add Condition Modal */}
      {isConditionModalOpen && (
        <div className="fixed inset-0 z-50 bg-slate-950/80 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="bg-slate-900 border border-slate-800 rounded-2xl max-w-md w-full p-6 space-y-4 shadow-2xl">
            <div className="flex justify-between items-center border-b border-slate-800 pb-3">
              <h3 className="text-base font-bold text-slate-100">Add Medical Condition</h3>
              <button onClick={() => setIsConditionModalOpen(false)} className="text-slate-400 hover:text-white">
                <X className="w-5 h-5" />
              </button>
            </div>

            <form onSubmit={handleConditionSubmit} className="space-y-4">
              <div>
                <label className="block text-xs font-medium text-slate-300 mb-1">Condition Name</label>
                <input
                  type="text"
                  value={conditionName}
                  onChange={(e) => setConditionName(e.target.value)}
                  placeholder="e.g. Essential Hypertension"
                  className="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg text-slate-200 text-xs"
                  required
                />
              </div>

              <div>
                <label className="block text-xs font-medium text-slate-300 mb-1">Notes</label>
                <textarea
                  value={conditionNotes}
                  onChange={(e) => setConditionNotes(e.target.value)}
                  placeholder="Clinical notes..."
                  className="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg text-slate-200 text-xs"
                  rows={3}
                />
              </div>

              <div className="flex justify-end space-x-3 pt-2">
                <button type="button" onClick={() => setIsConditionModalOpen(false)} className="px-4 py-2 bg-slate-800 text-slate-300 rounded-lg text-xs">
                  Cancel
                </button>
                <button type="submit" disabled={addConditionMutation.isPending} className="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 text-white rounded-lg text-xs font-semibold">
                  {addConditionMutation.isPending ? 'Saving...' : 'Save Condition'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Add Allergy Modal */}
      {isAllergyModalOpen && (
        <div className="fixed inset-0 z-50 bg-slate-950/80 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="bg-slate-900 border border-slate-800 rounded-2xl max-w-md w-full p-6 space-y-4 shadow-2xl">
            <div className="flex justify-between items-center border-b border-slate-800 pb-3">
              <h3 className="text-base font-bold text-rose-200">Record Patient Allergy</h3>
              <button onClick={() => setIsAllergyModalOpen(false)} className="text-slate-400 hover:text-white">
                <X className="w-5 h-5" />
              </button>
            </div>

            <form onSubmit={handleAllergySubmit} className="space-y-4">
              <div>
                <label className="block text-xs font-medium text-slate-300 mb-1">Allergen</label>
                <input
                  type="text"
                  value={allergen}
                  onChange={(e) => setAllergen(e.target.value)}
                  placeholder="e.g. Penicillin"
                  className="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg text-slate-200 text-xs"
                  required
                />
              </div>

              <div>
                <label className="block text-xs font-medium text-slate-300 mb-1">Severity</label>
                <select
                  value={allergySeverity}
                  onChange={(e) => setAllergySeverity(e.target.value)}
                  className="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg text-slate-200 text-xs"
                >
                  <option value="MILD">MILD</option>
                  <option value="MODERATE">MODERATE</option>
                  <option value="SEVERE">SEVERE</option>
                </select>
              </div>

              <div>
                <label className="block text-xs font-medium text-slate-300 mb-1">Reaction Description</label>
                <input
                  type="text"
                  value={allergyReaction}
                  onChange={(e) => setAllergyReaction(e.target.value)}
                  placeholder="e.g. Urticaria, Anaphylaxis"
                  className="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg text-slate-200 text-xs"
                />
              </div>

              <div className="flex justify-end space-x-3 pt-2">
                <button type="button" onClick={() => setIsAllergyModalOpen(false)} className="px-4 py-2 bg-slate-800 text-slate-300 rounded-lg text-xs">
                  Cancel
                </button>
                <button type="submit" disabled={addAllergyMutation.isPending} className="px-4 py-2 bg-rose-600 hover:bg-rose-500 text-white rounded-lg text-xs font-semibold">
                  {addAllergyMutation.isPending ? 'Saving...' : 'Record Allergy'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Add Lab Report Modal */}
      {isLabModalOpen && (
        <div className="fixed inset-0 z-50 bg-slate-950/80 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="bg-slate-900 border border-slate-800 rounded-2xl max-w-md w-full p-6 space-y-4 shadow-2xl">
            <div className="flex justify-between items-center border-b border-slate-800 pb-3">
              <h3 className="text-base font-bold text-slate-100">Add Laboratory Report</h3>
              <button onClick={() => setIsLabModalOpen(false)} className="text-slate-400 hover:text-white">
                <X className="w-5 h-5" />
              </button>
            </div>

            <form onSubmit={handleLabSubmit} className="space-y-4">
              <div>
                <label className="block text-xs font-medium text-slate-300 mb-1">Test Name</label>
                <input
                  type="text"
                  value={testName}
                  onChange={(e) => setTestName(e.target.value)}
                  placeholder="e.g. Fasting Blood Glucose"
                  className="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg text-slate-200 text-xs"
                  required
                />
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-xs font-medium text-slate-300 mb-1">Result</label>
                  <input
                    type="text"
                    value={result}
                    onChange={(e) => setResult(e.target.value)}
                    placeholder="126"
                    className="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg text-slate-200 text-xs"
                    required
                  />
                </div>

                <div>
                  <label className="block text-xs font-medium text-slate-300 mb-1">Unit</label>
                  <input
                    type="text"
                    value={unit}
                    onChange={(e) => setUnit(e.target.value)}
                    placeholder="mg/dL"
                    className="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg text-slate-200 text-xs"
                  />
                </div>
              </div>

              <div className="flex items-center space-x-2 pt-1">
                <input
                  type="checkbox"
                  id="abnormal"
                  checked={isAbnormal}
                  onChange={(e) => setIsAbnormal(e.target.checked)}
                  className="rounded border-slate-800 bg-slate-950 text-rose-500 focus:ring-rose-500"
                />
                <label htmlFor="abnormal" className="text-xs text-rose-400 font-medium">
                  Flag as Abnormal Result
                </label>
              </div>

              <div className="flex justify-end space-x-3 pt-2">
                <button type="button" onClick={() => setIsLabModalOpen(false)} className="px-4 py-2 bg-slate-800 text-slate-300 rounded-lg text-xs">
                  Cancel
                </button>
                <button type="submit" disabled={createLabMutation.isPending} className="px-4 py-2 bg-cyan-600 hover:bg-cyan-500 text-white rounded-lg text-xs font-semibold">
                  {createLabMutation.isPending ? 'Saving...' : 'Save Lab Report'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Prescription Creation Modal with Safety Warning Alert */}
      {isPrescriptionModalOpen && (
        <div className="fixed inset-0 z-50 bg-slate-950/80 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="bg-slate-900 border border-slate-800 rounded-2xl max-w-lg w-full p-6 space-y-4 shadow-2xl">
            <div className="flex justify-between items-center border-b border-slate-800 pb-3">
              <h3 className="text-base font-bold text-slate-100 flex items-center space-x-2">
                <Pill className="w-5 h-5 text-emerald-400" />
                <span>Issue Prescription</span>
              </h3>
              <button onClick={() => setIsPrescriptionModalOpen(false)} className="text-slate-400 hover:text-white">
                <X className="w-5 h-5" />
              </button>
            </div>

            {safetyAlertMsg && (
              <div className="p-3 bg-rose-950/80 border border-rose-800 rounded-xl space-y-2 text-xs text-rose-200">
                <div className="flex items-center space-x-2 font-bold text-rose-300">
                  <AlertOctagon className="w-5 h-5 text-rose-400 shrink-0" />
                  <span>CLINICAL SAFETY ALERT TRIGGERED</span>
                </div>
                <p className="text-[11px] leading-relaxed">{safetyAlertMsg}</p>
                <div className="pt-2 border-t border-rose-900/60">
                  <label className="block text-[10px] font-bold text-rose-300 uppercase mb-1">
                    Override Reason (Required to proceed)
                  </label>
                  <input
                    type="text"
                    value={overrideReason}
                    onChange={(e) => setOverrideReason(e.target.value)}
                    placeholder="Enter clinical rationale to override alert..."
                    className="w-full px-3 py-1.5 bg-slate-900 border border-rose-700/80 rounded text-slate-100 text-xs focus:ring-1 focus:ring-rose-500"
                  />
                </div>
              </div>
            )}

            <form onSubmit={handlePrescriptionSubmit} className="space-y-4">
              <div>
                <label className="block text-xs font-medium text-slate-300 mb-1">Select Medication</label>
                <select
                  value={selectedMedId}
                  onChange={(e) => setSelectedMedId(e.target.value)}
                  className="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg text-slate-200 text-xs"
                  required
                >
                  <option value="">-- Choose Medication --</option>
                  {medications?.map((m) => (
                    <option key={m.id} value={m.id}>
                      {m.name} ({m.generic_name})
                    </option>
                  ))}
                </select>
              </div>

              <div className="grid grid-cols-3 gap-3">
                <div>
                  <label className="block text-xs font-medium text-slate-300 mb-1">Dosage</label>
                  <input
                    type="text"
                    value={dosage}
                    onChange={(e) => setDosage(e.target.value)}
                    className="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg text-slate-200 text-xs"
                    required
                  />
                </div>

                <div>
                  <label className="block text-xs font-medium text-slate-300 mb-1">Frequency</label>
                  <input
                    type="text"
                    value={frequency}
                    onChange={(e) => setFrequency(e.target.value)}
                    className="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg text-slate-200 text-xs"
                    required
                  />
                </div>

                <div>
                  <label className="block text-xs font-medium text-slate-300 mb-1">Duration</label>
                  <input
                    type="text"
                    value={duration}
                    onChange={(e) => setDuration(e.target.value)}
                    className="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg text-slate-200 text-xs"
                    required
                  />
                </div>
              </div>

              <div className="flex justify-end space-x-3 pt-4 border-t border-slate-800">
                <button
                  type="button"
                  onClick={() => setIsPrescriptionModalOpen(false)}
                  className="px-4 py-2 bg-slate-800 text-slate-300 rounded-lg text-xs"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={createPrescriptionMutation.isPending}
                  className="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 text-white rounded-lg text-xs font-semibold"
                >
                  {createPrescriptionMutation.isPending ? 'Processing...' : safetyAlertMsg ? 'Override & Issue' : 'Issue Prescription'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  )
}
