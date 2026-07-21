<script setup lang="ts">
import { CircleCheck, CircleClose, Loading } from '@element-plus/icons-vue'

interface Step {
  title: string
  desc: string
}

const props = defineProps<{
  modelValue: boolean
  status: 'running' | 'success' | 'error'
  target: string
  progress: number
  elapsed: number
  steps: Step[]
  stepIndex: number
  tips: string[]
  tipIndex: number
  successSummary: string
  errorText: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  retry: []
  settings: []
  cancel: []
}>()

function close() {
  if (props.status !== 'running') emit('update:modelValue', false)
}
</script>

<template>
  <el-dialog
    :model-value="modelValue"
    :show-close="status !== 'running'"
    :close-on-click-modal="status !== 'running'"
    :close-on-press-escape="status !== 'running'"
    width="440px"
    align-center
    class="ai-dialog"
    @update:model-value="emit('update:modelValue', $event)"
    @close="close"
  >
    <template #header>
      <div class="ai-dlg-header">
        <span v-if="status === 'running'">正在组卷</span>
        <span v-else-if="status === 'success'">组卷完成</span>
        <span v-else>组卷失败</span>
      </div>
    </template>

    <div class="ai-dlg-body">
      <div class="ai-target">{{ target }}</div>

      <template v-if="status === 'running'">
        <el-progress :percentage="Math.round(progress)" :stroke-width="10" striped striped-flow />
        <div class="ai-elapsed">已用时 {{ elapsed }} 秒</div>

        <ul class="ai-steps">
          <li
            v-for="(step, index) in steps"
            :key="index"
            :class="{
              done: index < stepIndex,
              active: index === stepIndex,
              todo: index > stepIndex,
            }"
          >
            <span class="dot">
              <el-icon v-if="index < stepIndex"><CircleCheck /></el-icon>
              <el-icon v-else-if="index === stepIndex" class="spin"><Loading /></el-icon>
              <span v-else class="num">{{ index + 1 }}</span>
            </span>
            <div>
              <div class="st-title">{{ step.title }}</div>
              <div class="st-desc">{{ step.desc }}</div>
            </div>
          </li>
        </ul>

        <div class="ai-tip">{{ tips[tipIndex] }}</div>
      </template>

      <template v-else-if="status === 'success'">
        <div class="ai-result ok">
          <el-icon :size="36" color="#16a34a"><CircleCheck /></el-icon>
          <p>{{ successSummary }}</p>
        </div>
      </template>

      <template v-else>
        <div class="ai-result err">
          <el-icon :size="36" color="#dc2626"><CircleClose /></el-icon>
          <pre class="ai-err-text">{{ errorText }}</pre>
        </div>
      </template>
    </div>

    <template #footer>
      <template v-if="status === 'running'">
        <span class="ai-footer-hint">生成中可取消（网络请求可能仍会结束）</span>
        <el-button type="danger" plain @click="emit('cancel')">取消生成</el-button>
      </template>
      <el-button v-else-if="status === 'success'" type="primary" @click="emit('update:modelValue', false)">
        查看结果
      </el-button>
      <template v-else>
        <el-button @click="emit('update:modelValue', false)">关闭</el-button>
        <el-button @click="emit('settings')">接口设置</el-button>
        <el-button type="primary" @click="emit('retry')">重试</el-button>
      </template>
    </template>
  </el-dialog>
</template>
